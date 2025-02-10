use crate::core::filter::NameFilter;
use crate::http::chain::ReverseChain;
use crate::http::handlers::log::LogHandler;
use crate::http::handlers::proxy::ProxyHandler;
use crate::http::handlers::HttpHandler;
use axum::body::Body;
use axum::extract::State;
use axum::routing::any;
use axum::Router;
use clap::Parser;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::net::SocketAddr;
use std::sync::Arc;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "3249")]
    port: u16,
}

pub async fn run(args: &Args) -> anyhow::Result<()> {
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let handlers: Vec<Arc<dyn HttpHandler<Body>>> = vec![
        Arc::new(LogHandler::<Body>::new()),
        Arc::new(ProxyHandler::<Body, NameFilter>::new()),
    ];

    let chain = ReverseChain::new(handlers);
    let app = Router::new()
        .route(
            "/{*path}",
            any(any::<ReverseChain, (), State<Client>>(chain)),
        )
        .with_state(State(client));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
