use crate::runtime::secret::{SecretData, Storage};
use anyhow::bail;
use k8s_openapi::api::core::v1::{Secret, SecretReference};
use kube::Api;
use std::sync::Arc;

pub struct Kubernetes {
    pub namespace: String,
    pub name: String,
}

impl Kubernetes {
    pub fn new(namespace: String, name: String) -> Self {
        Self { namespace, name }
    }
}

impl Storage for Kubernetes {
    fn get(&self) -> SecretData {
        let namespace = self.namespace.clone();
        let name = self.name.clone();

        Box::pin(async move {
            if name.is_empty() {
                bail!("secretRef.name is required");
            }
            if namespace.is_empty() {
                bail!("secretRef.namespace is required");
            }

            let client = kube::Client::try_default().await?;
            let api = Api::<Secret>::namespaced(client, namespace.as_str());
            let secret = api.get(name.as_str()).await?;
            let data = secret.data.unwrap_or_default();

            let data = data
                .into_iter()
                .filter_map(|(k, v)| String::from_utf8(v.0).ok().map(|v| (k, v)))
                .collect();

            Ok(Arc::new(data))
        })
    }
}

impl Storage for SecretReference {
    fn get(&self) -> SecretData {
        let namespace = self
            .namespace
            .clone()
            .unwrap_or_else(|| String::from("default"));
        let name = self.name.clone().unwrap_or_default();

        Kubernetes::new(namespace, name).get()
    }
}
