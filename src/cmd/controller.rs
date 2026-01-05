use crate::api::proxy::Proxy;
use crate::api::script::Script;
use crate::cmd::{ProxyHandler, ResourceHandler, ScriptHandler};
use anyhow::{Result, anyhow};
use futures::TryStreamExt;
use k8s_openapi::NamespaceResourceScope;
use kube::runtime::watcher::Event;
use kube::{Api, Client, Resource, runtime::watcher};
use log::info;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use tokio::task::JoinSet;

async fn watch_resource<T, H>(handler: H) -> Result<()>
where
    T: Resource<Scope = NamespaceResourceScope>
        + DeserializeOwned
        + Clone
        + Debug
        + Send
        + Sync
        + 'static,
    H: ResourceHandler<T> + Send + Sync,
    <T as Resource>::DynamicType: Default,
{
    let client = Client::try_default().await?;
    let api = Api::<T>::all(client);
    let use_watchlist = std::env::var("WATCHLIST")
        .map(|s| s == "1")
        .unwrap_or(false);
    let wc = if use_watchlist {
        watcher::Config::default().streaming_lists()
    } else {
        watcher::Config::default()
    };

    watcher(api, wc)
        .try_for_each(|event| async {
            match event {
                Event::Init => {
                    info!("Pod watcher initialized");
                    Ok(())
                }
                Event::InitApply(t) | Event::Apply(t) => handler
                    .handle_create(t)
                    .await
                    .map_err(watcher::Error::WatchFailed),
                Event::Delete(t) => handler
                    .handle_delete(t)
                    .await
                    .map_err(watcher::Error::WatchFailed),

                Event::InitDone => {
                    info!("Initial pod list completed");
                    Ok(())
                }
            }
        })
        .await?;

    Ok(())
}

pub async fn run() -> Result<()> {
    let mut set: JoinSet<Result<()>> = JoinSet::new();
    let client = Client::try_default().await?;
    let proxy_client = client.clone();

    set.spawn(async move {
        let handler = ProxyHandler::new(proxy_client);
        watch_resource::<Proxy, _>(handler).await?;
        Ok(())
    });

    set.spawn(async move {
        let handler = ScriptHandler::new();
        watch_resource::<Script, _>(handler).await?;
        Ok(())
    });

    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(())) => continue,
            Ok(Err(e)) => {
                return Err(anyhow!("Task run failed: {}", e));
            }
            Err(e) => {
                return Err(anyhow!("Task join failed: {}", e));
            }
        }
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}
