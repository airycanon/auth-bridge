use crossbeam_skiplist::SkipMap;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use lazy_static::lazy_static;
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
