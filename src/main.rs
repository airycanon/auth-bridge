//! This example shows how one can begin with creating a MITM proxy.
//!
//! Note that this MITM proxy is not production ready, and is only meant
//! to show you how one might start. You might want to address the following:
//!
//! - Load in your tls mitm cert/key pair from file or ACME
//! - Make sure your clients trust the MITM cert
//! - Do not enforce the Application protocol and instead convert requests when needed,
//!   e.g. in this example we _always_ map the protocol between two ends,
//!   even though it might be better to be able to map bidirectionaly between http versions
//! - ... and much more
//!
//! That said for basic usage it does work and should at least give you an idea on how to get started.
//!
//! It combines concepts that can seen in action separately in the following examples:
//!
//! - [`http_connect_proxy`](./http_connect_proxy.rs);
//! - [`tls_boring_termination`](./tls_boring_termination.rs);
//!
//! # Run the example
//!
//! ```sh
//! cargo run --example http_mitm_proxy_boring --features=http-full,boring
//! ```
//!
//! ## Expected output
//!
//! The server will start and listen on `:62017`. You can use `curl` to interact with the service:
//!
//! ```sh
//! curl -v -x http://127.0.0.1:62017 --proxy-user 'john:secret' http://www.example.com/
//! curl -k -v -x http://127.0.0.1:62017 --proxy-user 'john:secret' https://www.example.com/
//! ```
//!
//! ## WebSocket support
//!
//! Since July of 2025 this example also contains WebSocket MITM support.
//! You can for example test it using:
//!
//! ```sh
//! rama ws -k \
//!     --proxy http://127.0.0.1:62017 --proxy-user 'john:secret' \
//!     wss://echo.ramaproxy.org
//! ```
//!
//! Or use one of alternative sub protocols available in the echo server:
//!
//! ```sh
//! rama ws -k \
//!     --proxy http://127.0.0.1:62017 --proxy-user 'john:secret' \
//!     --protocols echo-upper wss://echo.ramaproxy.org
//! ```

mod api;
mod cmd;
mod proxy;
mod runtime;

use clap::{Parser, Subcommand};
use kube::CustomResourceExt;
use serde_saphyr as yaml;
use std::fs;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::{proxy::Proxy, script::Script};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(long_about = "run controller")]
    Controller,
    #[command(long_about = "generate crd")]
    Crd {
        #[arg(long)]
        path: String,
    },
    #[command(long_about = "run forward http")]
    ForwardProxy(cmd::forward::Args),
    #[command(long_about = "run reverse http")]
    ReverseProxy(cmd::reverse::Args),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(
                    rama::telemetry::tracing::level_filters::LevelFilter::INFO.into(),
                )
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Controller => cmd::controller::run().await?,
        Commands::Crd { path } => write_crds(&path)?,
        Commands::ForwardProxy(args) => cmd::forward::run(&args).await?,
        Commands::ReverseProxy(args) => cmd::reverse::run(&args).await?,
    }

    Ok(())
}

fn write_crds(path: &str) -> anyhow::Result<()> {
    let mut out = String::new();
    out.push_str(&yaml::to_string(&Proxy::crd())?);
    out.push_str("\n---\n");
    out.push_str(&yaml::to_string(&Script::crd())?);
    fs::write(path, out)?;
    Ok(())
}
