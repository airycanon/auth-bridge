use crate::apis::auth::AuthMethod::Dynamic;
use crate::apis::proxy::Proxy;
use crate::apis::script::Script;
use crate::core::filter::ProxyFilter;
use crate::core::script::input::Input;
use crate::http::body::ProxyBody;
use anyhow::Result;
use bytes::Bytes;
use http::request::Parts;
use http::Uri;
use kube::api::ListParams;
use kube::{Api, Client, ResourceExt};
use log::{debug, error, info};
use serde_json::Value;

#[derive(Default)]
pub struct ProxyResolver {
    proxy: Option<Proxy>,
    policy_scripts: Vec<Script>,
    auth_script: Option<Script>,
}

impl ProxyResolver {
    pub async fn from_uri<F: ProxyFilter>(uri: Uri, filter: F) -> Result<Self> {
        let mut resolver = Self::default();

        let client = Client::try_default().await?;

        if let Some(proxy) = Self::get_proxy(&client, uri, &filter).await? {
            let namespace = proxy.namespace().unwrap_or("default".to_string());

            info!("proxy matched, proxy: {}/{}", namespace, proxy.name_any());

            let mut script_names: Vec<String> = proxy
                .spec
                .policies
                .iter()
                .map(|policy| policy.script.clone())
                .collect();

            if let Dynamic { ref script, .. } = proxy.spec.auth.method {
                script_names.push(script.clone())
            }
            let mut scripts = Self::get_scripts(&client, &script_names, namespace.as_str()).await?;

            if let Dynamic { ref script, .. } = proxy.spec.auth.method {
                resolver.auth_script = scripts
                    .iter()
                    .position(|s| s.name_any() == script.clone())
                    .map(|i| scripts.swap_remove(i));
            }
            resolver.policy_scripts = scripts;
            resolver.proxy = Some(proxy);
        }

        Ok(resolver)
    }

    async fn get_proxy<F: ProxyFilter>(
        client: &Client,
        uri: Uri,
        filter: &F,
    ) -> Result<Option<Proxy>> {
        let api = Api::<Proxy>::all(client.clone());
        let proxies = api.list(&ListParams::default()).await?;

        let target = proxies.into_iter().find(|proxy| filter.filter(proxy, &uri));

        Ok(target)
    }

    async fn get_scripts(
        client: &Client,
        names: &[String],
        namespace: &str,
    ) -> Result<Vec<Script>> {
        let api = Api::<Script>::namespaced(client.clone(), namespace);
        let policies = api.list(&ListParams::default()).await?;

        Ok(policies
            .into_iter()
            .filter(|policy| names.contains(&policy.name_any()))
            .collect())
    }

    pub fn evaluate(&self, input: &Input) -> Result<bool> {
        for script in &self.policy_scripts {
            let engine = script.spec.engine.get_executor();
            let result = match engine.execute(script.spec.source.clone(), input)? {
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

    pub async fn apply<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> Result<ProxyBody> {
        if let Some(Proxy { spec, .. }) = &self.proxy {
            let driver = spec.auth.storage.driver()?;
            let secret_data = driver.get().await?;

            if !spec.address.eq(&parts.uri) {
                let old = parts.uri.clone();
                parts.uri = spec.address.replace(&parts.uri)?;
                debug!("replace target uri, old: {}, new: {}", old, parts.uri)
            }

            let injector = spec
                .auth
                .method
                .injector(&secret_data, self.auth_script.clone())?;

            debug!("injector created: {:?}", injector);

            Ok(injector.inject(parts, bytes).await?)
        } else {
            Ok(ProxyBody::from(bytes))
        }
    }
}
