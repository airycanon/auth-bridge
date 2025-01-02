use crate::core::filter::NameFilter;
use crate::http::chain::Chain;
use crate::http::log::{log_request, log_response};
use crate::http::proxy::proxy_request;
use axum::body::Body;
use axum::extract::State;
use axum::routing::any;
use axum::Router;
use clap::Parser;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::net::SocketAddr;

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

    let chain = Chain::new()
        .with_request_handler(log_request)
        .with_request_handler(proxy_request::<_, NameFilter>)
        .with_response_handler(log_response);

    let app = Router::new()
        .route("/*path", any(any::<Chain<Body>, (), State<Client>>(chain)))
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
