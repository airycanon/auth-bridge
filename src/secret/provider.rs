use std::borrow::Cow;
use std::collections::BTreeMap;
use anyhow::{anyhow, Result, Ok};
use k8s_openapi::api::core::v1::Secret;
use kube::{Api, Client};
use crate::apis::proxy_policy::{ProxyPolicyAuth};
use crate::secret::provider::Provider::{Kubernetes, Raw};

pub enum Provider {
    Kubernetes {
        namespace: String,
        name: String,
    },
    Raw(BTreeMap<String, String>),
}

impl Provider {
    pub async fn secret(&self) -> Result<Cow<'_, BTreeMap<String, String>>> {
        let data = match self {
            Kubernetes { namespace, name } => Cow::Owned(kubernetes_secret(namespace, name).await?),
            Raw(data) => Cow::Borrowed(data),
        };
        Ok(data)
    }
}

async fn kubernetes_secret(namespace: &str, name: &str) -> Result<BTreeMap<String, String>> {
    let client = Client::try_default().await?;
    let api = Api::<Secret>::namespaced(client, namespace);
    let secret = api.get(name).await?;
    let data = secret.data.unwrap_or_default();

    let data = data.into_iter().map(|(k, v)| {
        let v = String::from_utf8(v.0).unwrap();
        (k, v)
    }).collect();

    Ok(data)
}

pub fn provider(auth: &ProxyPolicyAuth) -> Result<Provider> {
    let auth = auth.clone();
    match (auth.secret.reference, auth.secret.raw) {
        (Some(obj), None) => {
            let namespace = obj.namespace.ok_or(anyhow!("namespace required"))?;
            let name = obj.name.ok_or(anyhow!("name required"))?;
            Ok(Kubernetes { namespace, name })
        }
        (None, Some(data)) => {
            Ok(Raw(data))
        }
        _ => unreachable!()
    }
}
