use crate::apis::auth::Auth;
use crate::apis::policy::Policy;
use anyhow::Result;
use futures::stream::StreamExt;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use k8s_openapi::NamespaceResourceScope;
use kube::{
    api::PostParams,
    runtime::{watcher, WatchStreamExt},
    Api, Client, CustomResourceExt, Resource, ResourceExt,
};
use log::{debug, error, info};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

pub async fn run() -> Result<()> {
    let client = Client::try_default().await?;
    let crd_api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let params = PostParams::default();

    let mut policy = Policy::crd();
    apply_crd(&crd_api, &mut policy, &params).await?;

    let mut auth = Auth::crd();
    apply_crd(&crd_api, &mut auth, &params).await?;

    tokio::spawn(async move {
        let policy_handler = |policy: Policy| -> Result<()> {
            info!("Handling ProxyPolicy: {:?}", policy.spec);
            Ok(())
        };

        if let Err(e) = watch_resource::<Policy>(policy_handler).await {
            error!("ProxyPolicy watcher error: {}", e);
        }
    });

    tokio::spawn(async move {
        let auth_handler = |auth: Auth| -> Result<()> {
            info!("Handling ProxyAuth: {:?}", auth.spec);
            Ok(())
        };

        if let Err(e) = watch_resource::<Auth>(auth_handler).await {
            error!("ProxyAuth watcher error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn apply_crd(
    crd_api: &Api<CustomResourceDefinition>,
    crd: &mut CustomResourceDefinition,
    params: &PostParams,
) -> Result<(), kube::Error>
{
    match crd_api.get(crd.meta().name.as_ref().unwrap()).await {
        Ok(old_crd) => {
            crd.meta_mut().resource_version = old_crd.metadata.resource_version;
            match crd_api
                .replace(crd.meta().name.as_ref().unwrap(), params, crd)
                .await
            {
                Ok(o) => info!("Updated CRD: {} ({:?})", o.name_any(), o.status.unwrap()),
                Err(e) => error!("Failed to update CRD: {}", e),
            }
        }
        Err(kube::Error::Api(err_resp)) if err_resp.code == 404 => {
            match crd_api.create(params, crd).await {
                Ok(o) => {
                    info!("Created {} ({:?})", o.name_any(), o.status.unwrap());
                    debug!("Created CRD: {:?}", o.spec);
                }
                Err(e) => return Err(e),
            }
        }
        Err(e) => {
            error!("Failed to retrieve existing CRD: {}", e);
            return Err(e);
        }
    }
    Ok(())
}

async fn watch_resource<T>(handler: impl Fn(T) -> Result<()> + Send + 'static) -> Result<()>
where
    T: Resource<Scope = NamespaceResourceScope>
        + DeserializeOwned
        + Clone
        + Debug
        + Send
        + Sync
        + 'static,
    <T as Resource>::DynamicType: Default,
{
    let client = Client::try_default().await?;
    let api = Api::<T>::default_namespaced(client);
    let use_watchlist = std::env::var("WATCHLIST")
        .map(|s| s == "1")
        .unwrap_or(false);
    let wc = if use_watchlist {
        watcher::Config::default().streaming_lists()
    } else {
        watcher::Config::default()
    };

    let mut stream = watcher(api, wc).applied_objects().boxed();
    while let Some(event) = stream.next().await {
        match event {
            Ok(p) => {
                if let Err(e) = handler(p) {
                    error!("Handler error: {}", e);
                }
            }
            Err(e) => error!("Watch error: {}", e),
        }
    }
    Ok(())
}
