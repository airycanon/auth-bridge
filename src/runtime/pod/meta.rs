use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use serde::Serialize;
use std::collections::BTreeMap;

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
