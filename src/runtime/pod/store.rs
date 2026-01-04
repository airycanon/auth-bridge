use anyhow::Result;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::watcher;
use kube::runtime::watcher::Event;
use kube::{Api, Client, ResourceExt};
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;



use once_cell::sync::Lazy;
use crate::runtime::pod::meta::Meta;

static STORE: Lazy<Store> = Lazy::new(Store::default);

#[derive(Default)]
pub struct Store {
    metas: Arc<RwLock<HashMap<String, Arc<Meta>>>>,
}

impl Store {
    pub fn global() -> &'static Lazy<Store, fn() -> Store> {
        &STORE
    }

    pub fn find(&self, ip: &String) -> Option<Arc<Meta>> {
        self.metas
            .read()
            .ok()
            .and_then(|metas| metas.get(ip).map(Arc::clone))
    }

    pub fn insert(&self, pod: &mut Pod) {
        if pod.metadata.deletion_timestamp.is_some() {
            return;
        }

        pod.annotations_mut()
            .remove("kubectl.kubernetes.io/last-applied-configuration");

        if let Some(ip) = self.get_pod_ip(pod) {
            if let Ok(mut metas) = self.metas.write() {
                if !metas.contains_key(&ip) {
                    let meta = Meta::from(&*pod);
                    info!(
                        "pod {:?} added with IP: {:?}",
                        (&meta.namespace, &meta.name),
                        &ip
                    );
                    metas.insert(ip, Arc::new(meta));
                }
            }
        }
    }

    pub fn delete(&self, pod: &Pod) {
        if let Some(ip) = self.get_pod_ip(pod) {
            if let Ok(mut metas) = self.metas.write() {
                if let Some(meta) = metas.remove(&ip) {
                    info!(
                        "pod {:?} deleted with IP: {:?}",
                        (&meta.namespace, &meta.name),
                        &ip
                    );
                }
            }
        }
    }

    fn get_pod_ip(&self, pod: &Pod) -> Option<String> {
        pod.status
            .as_ref()
            .and_then(|status| status.pod_ip.as_ref())
            .cloned()
    }

    pub async fn watch(&self) -> Result<()> {
        let client = Client::try_default().await?;
        let api = Api::<Pod>::all(client);
        let watcher = watcher(api, watcher::Config::default());

        watcher
            .try_for_each(|event| async move {
                match event {
                    Event::Init => {
                        info!("Pod watcher initialized");
                    }
                    Event::InitApply(mut pod) | Event::Apply(mut pod) => self.insert(&mut pod),
                    Event::Delete(ref pod) => self.delete(pod),
                    Event::InitDone => {
                        info!("Initial pod list completed");
                    }
                }
                Ok(())
            })
            .await?;
        Ok(())
    }
}
