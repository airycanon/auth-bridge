use crate::apis::proxy::{Address, Proxy, ProxyService};
use crate::apis::script::Script;
use crate::core::env::{FORWARD_PROXY_ENV, REVERSE_PROXY_ENV};
use futures::future::BoxFuture;
use k8s_openapi::api::core::v1::{Service, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{DeleteParams, ListParams, Patch, PatchParams, PostParams};
use kube::core::Selector;
use kube::Error::Api as ApiError;
use kube::{Api, Client, ResourceExt};
use log::{debug, info, warn};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::string::ToString;

pub mod controller;
pub mod forward;
pub mod reverse;

const PROXY_LABEL: &str = "auth-bridge.dev/proxy";

trait ResourceHandler<T>
where
    T: Debug,
{
    fn handle_create(&self, resource: T) -> BoxFuture<Result<(), kube::Error>> {
        debug!("handle resource change: {:?}", resource);
        Box::pin(async move { Ok(()) })
    }
    fn handle_delete(&self, resource: T) -> BoxFuture<Result<(), kube::Error>> {
        debug!("handle resource delete: {:?}", resource);
        Box::pin(async move { Ok(()) })
    }
}

struct ProxyHandler {
    client: Client,
}

impl ProxyHandler {
    fn new(client: Client) -> Self {
        Self { client }
    }

    async fn create_service(
        &self,
        proxy_namespace: &str,
        proxy_name: &str,
    ) -> Result<Option<Service>, kube::Error> {
        let api = Api::<Service>::namespaced(self.client.clone(), proxy_namespace);

        let label = BTreeMap::from([(PROXY_LABEL.to_string(), proxy_name.to_string())]);

        match api.get(proxy_name).await {
            Ok(service) => {
                if service.labels().get(PROXY_LABEL) != Some(&proxy_name.to_string()) {
                    warn!("service existing with no label: {}", proxy_name)
                }
                Ok(None)
            }
            Err(ApiError(resp)) if resp.code == http::StatusCode::NOT_FOUND => {
                let reverse_proxy = ProxyService::from_env(REVERSE_PROXY_ENV);

                let service = Service {
                    metadata: ObjectMeta {
                        labels: Some(label),
                        name: Some(proxy_name.to_string()),
                        namespace: Some(proxy_namespace.to_string()),
                        ..ObjectMeta::default()
                    },
                    spec: Some(ServiceSpec {
                        type_: Some("ExternalName".to_string()),
                        external_name: Some(reverse_proxy.endpoint),
                        ..ServiceSpec::default()
                    }),
                    status: None,
                };
                info!("create service for proxy: {}", proxy_name);

                Ok(Some(api.create(&PostParams::default(), &service).await?))
            }
            Err(error) => Err(error),
        }
    }

    async fn update_address(
        &self,
        namespace: &String,
        name: &String,
        service: &String,
    ) -> Result<(), kube::Error> {
        let api = Api::<Proxy>::namespaced(self.client.clone(), namespace.as_str());

        let forward_proxy = ProxyService::from_env(FORWARD_PROXY_ENV);
        let reverse_proxy = ProxyService::new(namespace.to_string(), service.to_string());

        let patch = json!({
            "status": {
                "address": Address {
                    forward: forward_proxy,
                    reverse: reverse_proxy,
                },
            },
        });
        api.patch_status(
            name.as_str(),
            &PatchParams::default(),
            &Patch::Merge(&patch),
        )
        .await?;

        Ok(())
    }
}

impl ResourceHandler<Proxy> for ProxyHandler {
    fn handle_create(&self, proxy: Proxy) -> BoxFuture<Result<(), kube::Error>> {
        debug!("handle proxy: {:?}", proxy);

        Box::pin(async move {
            let namespace = proxy.namespace().unwrap();
            let name = proxy.name_any();
            if let Some(service) = self.create_service(&namespace, &name).await? {
                self.update_address(&namespace, &name, &service.name_any())
                    .await?;
            }

            Ok(())
        })
    }

    fn handle_delete(&self, proxy: Proxy) -> BoxFuture<Result<(), kube::Error>> {
        Box::pin(async move {
            let api = Api::<Service>::namespaced(
                self.client.clone(),
                proxy.namespace().unwrap().as_str(),
            );

            let mut label = BTreeMap::new();
            label.insert(PROXY_LABEL.to_string(), proxy.name_any());

            let selector = Selector::from_iter(label.clone().into_iter());
            let delete_params = DeleteParams::default();
            let list_params = ListParams::default().labels_from(&selector);

            api.delete_collection(&delete_params, &list_params).await?;

            Ok(())
        })
    }
}

struct ScriptHandler {}

impl ScriptHandler {
    fn new() -> Self {
        Self {}
    }
}

impl ResourceHandler<Script> for ScriptHandler {}
