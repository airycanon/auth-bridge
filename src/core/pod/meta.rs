use crossbeam_skiplist::SkipMap;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use lazy_static::lazy_static;
use log::info;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

lazy_static! {
    static ref PODMETAS: SkipMap<String, Arc<PodMeta>> = SkipMap::new();
}

#[derive(Serialize)]
pub struct PodMeta {
    pub name: String,
    pub namespace: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
}

impl From<&Pod> for PodMeta {
    fn from(pod: &Pod) -> Self {
        PodMeta {
            namespace: pod.namespace().unwrap(),
            name: pod.name_any(),
            labels: pod.labels().clone(),
            annotations: pod.annotations().clone(),
        }
    }
}

pub fn find(ip: &String) -> Option<Arc<PodMeta>> {
    if let Some(entry) = PODMETAS.get(ip) {
        return Some(Arc::clone(entry.value()));
    }
    None
}

pub fn bind(pod: &Pod) {
    if pod.metadata.deletion_timestamp.is_some() {
        return;
    }

    let meta = PodMeta::from(pod);
    if let Some(ip) = get_pod_ip(pod) {
        if !PODMETAS.contains_key(&ip) {
            info!(
                "pod {:?} added with IP: {:?}",
                (&meta.namespace, &meta.name),
                &ip
            );
            PODMETAS.insert(ip, Arc::new(meta));
        }
    }
}

pub fn unbind(pod: &Pod) {
    if let Some(ip) = get_pod_ip(pod) {
        if let Some(entry) = PODMETAS.get(&ip) {
            let meta = entry.value();
            info!(
                "pod {:?} deleted with IP: {:?}",
                (&meta.namespace, &meta.name),
                &ip
            );
            entry.remove();
        }
    }
}

fn get_pod_ip(pod: &Pod) -> Option<String> {
    let ip = pod
        .status
        .as_ref()
        .and_then(|status| status.pod_ip.as_ref())?;
    Some(ip.clone())
}
