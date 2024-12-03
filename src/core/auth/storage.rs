use anyhow::Result;
use k8s_openapi::api::core::v1::{ObjectReference, Secret};
use kube::Api;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

type SecretData<'a> = Pin<Box<dyn Future<Output = Result<BTreeMap<String, String>>> + Send + 'a>>;

pub trait Storage: Send {
    fn get(&self) -> SecretData;
}

pub struct Raw(pub BTreeMap<String, String>);

impl Storage for Raw {
    fn get(&self) -> SecretData {
        Box::pin(async move { Ok(self.0.clone()) })
    }
}

pub struct Kubernetes {
    pub namespace: String,
    pub name: String,
}

impl Kubernetes {
    pub fn new(secret_ref: ObjectReference) -> Self {
        let namespace = secret_ref.namespace.unwrap_or_default();
        let name = secret_ref.name.unwrap_or_default();

        Self { namespace, name }
    }
}

impl Storage for Kubernetes {
    fn get(&self) -> SecretData {
        Box::pin(async move {
            let client = kube::Client::try_default().await?;
            let api = Api::<Secret>::namespaced(client, self.namespace.as_str());
            let secret = api.get(self.name.as_str()).await?;
            let data = secret.data.unwrap_or_default();

            Ok(data
                .into_iter()
                .filter_map(|(k, v)| String::from_utf8(v.0).ok().map(|v| (k, v)))
                .collect())
        })
    }
}
