use crate::proxy::layers::{DecisionLayer, InjectLayer};
use crate::proxy::service::{
    ProxyState, http_connect_accept, http_connect_proxy, new_http_proxy, new_tls_acceptor,
};
use crate::runtime::pod::store::Store;
use crate::runtime::support::filter::AddressFilter;
use anyhow::{Error, Result};
use clap::Parser;
use log::error;
use rama::{
    Layer,
    http::{
        layer::{trace::TraceLayer, upgrade::UpgradeLayer},
        matcher::MethodMatcher,
        server::HttpServer,
    },
    layer::{AddInputExtensionLayer, ConsumeErrLayer},
    net::stream::layer::http::BodyLimitLayer,
    rt::Executor,
    service::service_fn,
    tcp::server::TcpListener,
};
use std::{sync::Arc, time::Duration};
use tokio::spawn;

#[derive(Parser, Debug)]
pub struct Args {
    /// path of the ca key
    #[arg(long, default_value = "ca.key")]
    ca_key: String,

    /// path of the ca cert
    #[arg(long, default_value = "ca.cert")]
    ca_cert: String,

    #[arg(long, default_value = "3149")]
    port: u16,
}

pub async fn run(args: &Args) -> Result<()> {
    spawn(async move {
        if let Err(error) = Store::global().watch().await {
            error!("Failed to watch pods: {}", error);
            std::process::exit(1);
        }
    });

    let tls_acceptor =
        new_tls_acceptor(args.ca_key.clone(), args.ca_cert.clone()).map_err(Error::msg)?;

    let state = ProxyState {
        tls_acceptor: Some(tls_acceptor),
        user_agent: Arc::new(
            rama::ua::profile::UserAgentDatabase::try_embedded().map_err(Error::msg)?,
        ),
    };

    let graceful = rama::graceful::Shutdown::default();
    let port = args.port;

    graceful.spawn_task_fn(move |guard| async move {
        let exec = Executor::graceful(guard.clone());
        let tcp_service = TcpListener::build(exec.clone())
            .bind_address(format!("0.0.0.0:{port}"))
            .await
            .expect("bind tcp proxy");
        let layers = (DecisionLayer::new(AddressFilter), InjectLayer);

        let http_mitm_service = new_http_proxy(&state, layers.clone());

        let http_service = (
            TraceLayer::new_for_http(),
            ConsumeErrLayer::default(),
            UpgradeLayer::new(
                exec.clone(),
                MethodMatcher::CONNECT,
                service_fn(http_connect_accept),
                service_fn(move |upgraded| http_connect_proxy(upgraded, layers.clone())),
            ),
        )
            .into_layer(http_mitm_service);

        let http_service = HttpServer::auto(exec).service(http_service);

        tcp_service
            .serve(
                (
                    AddInputExtensionLayer::new(state),
                    BodyLimitLayer::symmetric(2 * 1024 * 1024),
                )
                    .into_layer(http_service),
            )
            .await;
    });

    graceful
        .shutdown_with_limit(Duration::from_secs(30))
        .await
        .map_err(Error::from)?;

    Ok(())
}
