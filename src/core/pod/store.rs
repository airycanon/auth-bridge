use anyhow::Result;
use crossbeam_skiplist::SkipMap;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::watcher;
use kube::runtime::watcher::Event;
use kube::{Api, Client, ResourceExt};
use log::info;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Serialize)]
pub struct Meta {
    pub name: String,
    pub namespace: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
}

impl From<&Pod> for Meta {
    fn from(pod: &Pod) -> Self {
        Meta {
            namespace: pod.namespace().unwrap(),
            name: pod.name_any(),
            labels: pod.labels().clone(),
            annotations: pod.annotations().clone(),
        }
    }
}

use once_cell::sync::Lazy;

static STORE: Lazy<Store> = Lazy::new(Store::default);

#[derive(Default)]
pub struct Store {
    metas: Arc<SkipMap<String, Arc<Meta>>>,
}

impl Store {
    pub fn global() -> &'static Lazy<Store, fn() -> Store> {
        &STORE
    }

    pub fn find(&self, ip: &String) -> Option<Arc<Meta>> {
        self.metas.get(ip).map(|entry| Arc::clone(entry.value()))
    }

    pub fn insert(&self, pod: &Pod) {
        if pod.metadata.deletion_timestamp.is_some() {
            return;
        }

        if let Some(ip) = self.get_pod_ip(pod) {
            if !self.metas.contains_key(&ip) {
                let meta = Meta::from(pod);
                info!(
                    "pod {:?} added with IP: {:?}",
                    (&meta.namespace, &meta.name),
                    &ip
                );
                self.metas.insert(ip, Arc::new(meta));
            }
        }
    }

    pub fn delete(&self, pod: &Pod) {
        if let Some(ip) = self.get_pod_ip(pod) {
            if let Some(entry) = self.metas.remove(&ip) {
                let meta = entry.value();
                info!(
                    "pod {:?} deleted with IP: {:?}",
                    (&meta.namespace, &meta.name),
                    &ip
                );
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
            .try_for_each(|event| {
                async move {
                    match event {
                        Event::Init => {
                            info!("Pod watcher initialized");
                        }
                        Event::InitApply(pod) | Event::Apply(pod) => self.insert(&pod),
                        Event::Delete(pod) => self.delete(&pod),
                        Event::InitDone => {
                            info!("Initial pod list completed");
                        }
                    }
                    Ok(())
                }
            })
            .await?;
        Ok(())
    }
}
