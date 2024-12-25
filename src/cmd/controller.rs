use anyhow::Result;
use futures::stream::StreamExt;
use k8s_openapi::NamespaceResourceScope;
use kube::{
    runtime::{watcher, WatchStreamExt},
    Api, Client, Resource,
};
use log::{error, info};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use crate::apis::proxy::Proxy;
use crate::apis::script::Script;

pub async fn run() -> Result<()> {
    tokio::spawn(async move {
        let proxy_handler = |proxy: Proxy| -> Result<()> {
            info!("Handling Proxy: {:?}", proxy.spec);
            Ok(())
        };

        if let Err(e) = watch_resource::<Proxy>(proxy_handler).await {
            error!("ProxyAuth watcher error: {}", e);
        }
    });

    tokio::spawn(async move {
        let script_handler = |script: Script| -> Result<()> {
            info!("Handling ProxyPolicy: {:?}", script.spec);
            Ok(())
        };

        if let Err(e) = watch_resource::<Script>(script_handler).await {
            error!("ProxyPolicy watcher error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;

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
