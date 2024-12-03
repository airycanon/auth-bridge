use crate::proxy::reverse::proxy;
use axum::body::Body;
use axum::routing::any;
use axum::Router;
use clap::Parser;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::net::SocketAddr;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "4000")]
    port: u16,
}

pub async fn run(args: &Args) -> anyhow::Result<()> {
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let app = Router::new()
        .route("/", any(proxy::handler))
        .with_state(client);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", args.port)).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
