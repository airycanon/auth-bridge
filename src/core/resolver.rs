use crate::apis::auth::Auth;
use crate::apis::policy::Policy;
use crate::apis::proxy::Proxy;
use crate::core::body::ProxyBody;
use crate::core::filter::ProxyFilter;
use crate::core::script::input::Input;
use anyhow::Result;
use bytes::Bytes;
use http::request::Parts;
use http::Uri;
use kube::api::ListParams;
use kube::{Api, Client, ResourceExt};
use log::error;
use serde_json::Value;

#[derive(Default)]
pub struct ProxyResolver {
    proxy: Option<Proxy>,
    auth: Option<Auth>,
    policies: Vec<Policy>,
}

impl ProxyResolver {
    pub async fn from_uri<F: ProxyFilter>(uri: Uri, filter: F) -> Result<Self> {
        let client = Client::try_default().await?;

        if let Some(proxy) = Self::get_proxy(&client, uri, &filter).await? {
            let auth = Self::get_auth(&client, &proxy.spec.auth.name).await?;

            let policy_names: Vec<String> = proxy
                .spec
                .policies
                .iter()
                .map(|policy| policy.name.clone())
                .collect();
            let policies = Self::get_policies(&client, &policy_names).await?;

            Ok(Self {
                proxy: Some(proxy),
                auth: Some(auth),
                policies,
            })
        } else {
            Ok(Self::default())
        }
    }

    async fn get_proxy<F: ProxyFilter>(
        client: &Client,
        uri: Uri,
        filter: &F,
    ) -> Result<Option<Proxy>> {
        let api = Api::<Proxy>::all(client.clone());
        let proxies = api.list(&ListParams::default()).await?;
        Ok(proxies.into_iter().find(|proxy| filter.filter(proxy, &uri)))
    }

    async fn get_auth(client: &Client, name: &str) -> Result<Auth> {
        let api = Api::<Auth>::all(client.clone());
        Ok(api.get(name).await?)
    }

    async fn get_policies(client: &Client, names: &[String]) -> Result<Vec<Policy>> {
        let api = Api::<Policy>::all(client.clone());
        let policies = api.list(&ListParams::default()).await?;

        Ok(policies
            .into_iter()
            .filter(|policy| names.contains(&policy.name_any()))
            .collect())
    }

    pub fn evaluate(&self, input: &Input) -> Result<bool> {
        for policy in &self.policies {
            let engine = policy.spec.engine.get_executor();
            let result = match engine.execute(policy.spec.script.clone(), input)? {
                Value::String(value) => value == "true",
                Value::Bool(value) => value,
                value => {
                    error!("unsupported value {}", value);
                    false
                }
            };

            if !result {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn apply(&self, parts: Parts, bytes: Bytes) -> Result<(Parts, ProxyBody)> {
        match (&self.proxy, &self.auth) {
            (Some(proxy), Some(auth)) => {
                let driver = proxy.spec.auth.storage.driver()?;
                let secret_data = driver.get().await?;

                let injector = auth.spec.method.injector(&secret_data)?;
                Ok(injector.inject(parts, bytes).await?)
            }
            _ => Ok((parts, ProxyBody::from(bytes))),
        }
    }
}
