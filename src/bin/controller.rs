use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    CustomResourceExt, Client, Api, ResourceExt,
    api::PostParams,
    runtime::{watcher, WatchStreamExt},
};
use log::{debug, error, info};
use anyhow::Result;
use futures::stream::StreamExt;
use auth_bridge::apis::proxy_policy::ProxyPolicy;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;

    // Manage CRDs first
    let crd_api: Api<CustomResourceDefinition> = Api::all(client.clone());

    let mut crd = ProxyPolicy::crd();
    let params = PostParams::default();

    match crd_api.get(&crd.metadata.name.as_ref().unwrap()).await {
        Ok(old_crd) => {
            crd.metadata.resource_version = old_crd.metadata.resource_version;
            match crd_api.replace(&crd.metadata.name.as_ref().unwrap(), &params, &crd).await {
                Ok(o) => info!("Updated CRD: {} ({:?})", o.name_any(), o.status.unwrap()),
                Err(e) => error!("Failed to update CRD: {}", e),
            }
        }
        Err(kube::Error::Api(err_resp)) => if err_resp.code == 404 {
            match crd_api.create(&params, &crd).await {
                Ok(o) => {
                    info!("Created {} ({:?})", o.name_any(), o.status.unwrap());
                    debug!("Created CRD: {:?}", o.spec);
                }
                Err(e) => return Err(e.into()),
            }
        },
        Err(e) => error!("Failed to retrieve existing CRD: {}", e),
    }

    let client = Client::try_default().await?;
    let api = Api::<ProxyPolicy>::default_namespaced(client);
    let use_watchlist = std::env::var("WATCHLIST").map(|s| s == "1").unwrap_or(false);
    let wc = if use_watchlist {
        // requires WatchList feature gate on 1.27 or later
        watcher::Config::default().streaming_lists()
    } else {
        watcher::Config::default()
    };

    let mut stream = watcher(api, wc).applied_objects().boxed();
    while let Some(event) = stream.next().await {
        match event {
            Ok(p) => {
                info!("saw {:?}", p.spec);
            }
            Err(e) => error!("watch error: {}", e),
        }
    }

    Ok(())
}
