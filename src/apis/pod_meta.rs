use crossbeam_skiplist::SkipMap;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use lazy_static::lazy_static;
use log::info;
use std::collections::BTreeMap;
use std::sync::Arc;
use regorus::Value;

lazy_static! {
    static ref PODMETAS: SkipMap<String, Arc<PodMeta>> = SkipMap::new();
}

pub struct PodMeta {
    pub name: String,
    pub namespace: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
}

impl PodMeta {
    pub fn from(pod: &Pod) -> Self {
        PodMeta {
            namespace: pod.namespace().unwrap(),
            name: pod.name_any(),
            labels: pod.labels().clone(),
            annotations: pod.annotations().clone(),
        }
    }

    pub fn as_input(&self) -> Value {
        let mut input: BTreeMap<Value, Value> = BTreeMap::new();
        input.insert(Value::from("name"), Value::from(self.name.as_str()));
        input.insert(Value::from("namespace"), Value::from(self.namespace.as_str()));
        let labels: BTreeMap<Value, Value> = self.labels.clone()
            .into_iter()
            .map(|(k, v)| (Value::from(k), Value::from(v)))
            .collect();
        input.insert(Value::from("labels"), Value::from(labels));

        let annotations: BTreeMap<Value, Value> = self.annotations.clone()
            .into_iter()
            .map(|(k, v)| (Value::from(k), Value::from(v)))
            .collect();
        input.insert(Value::from("annotations"), Value::from(annotations));

        Value::from(input)
    }
}

pub fn find(ip: &String) -> Option<Arc<PodMeta>> {
    if let Some(entry) = PODMETAS.get(ip) {
        return Some(Arc::clone(entry.value()));
    }
    None
}

pub fn bind(pod: &Pod) {
    let meta = PodMeta::from(pod);
    if let Some(ip) = get_pod_ip(pod) {
        info!("pod {:?} added with IP: {:?}", (&meta.namespace, &meta.name), &ip);
        PODMETAS.insert(ip, Arc::new(meta));
    }
}

pub fn bind_all(pods: Vec<Pod>) {
    PODMETAS.clear();
    for pod in pods {
        bind(&pod);
    }
}

pub fn unbind(pod: &Pod) {
    if let Some(ip) = get_pod_ip(pod) {
        if let Some(entry) = PODMETAS.get(&ip) {
            let meta = entry.value();
            info!("pod {:?} deleted with IP: {:?}", (&meta.namespace, &meta.name), &ip);
            entry.remove();
        }
    }
}

fn get_pod_ip(pod: &Pod) -> Option<String> {
    let ip = pod.status.as_ref().and_then(|status| status.pod_ip.as_ref())?;
    Some(ip.clone())
}