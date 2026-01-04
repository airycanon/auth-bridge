use crate::runtime::support::filter::NameFilter;
use crate::proxy::layers::decision::DecisionLayer;
use crate::proxy::layers::inject::InjectLayer;
use crate::proxy::layers::normalize::NormalizeLayer;
use crate::proxy::service::{ProxyState, new_http_proxy};
use anyhow::Error;
use clap::Parser;
use rama::{
    Layer, http::server::HttpServer, layer::AddInputExtensionLayer,
    net::stream::layer::http::BodyLimitLayer, rt::Executor, tcp::server::TcpListener,
};
use std::{sync::Arc, time::Duration};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "3249")]
    port: u16,
}

pub async fn run(args: &Args) -> anyhow::Result<()> {
    let state = ProxyState {
        tls_acceptor: None,
        user_agent: Arc::new(rama::ua::profile::UserAgentDatabase::try_embedded()?),
    };

    let graceful = rama::graceful::Shutdown::default();
    let port = args.port;
    let reverse_state = state.clone();

    graceful.spawn_task_fn(move |guard| async move {
        let tcp_service = TcpListener::build()
            .bind(format!("0.0.0.0:{port}"))
            .await
            .expect("bind reverse proxy");

        let exec = Executor::graceful(guard.clone());
        let layers = (
            NormalizeLayer::default(),
            DecisionLayer::new(NameFilter::default()),
            InjectLayer::default(),
        );
        let http_reverse_service = new_http_proxy(&reverse_state, layers);
        let http_service = HttpServer::auto(exec).service(http_reverse_service);

        tcp_service
            .serve_graceful(
                guard,
                (
                    AddInputExtensionLayer::new(reverse_state),
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
