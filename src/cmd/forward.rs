use crate::core::pod::store::Store;
use crate::proxy::forward::log::LogHandler;
use crate::proxy::forward::multi::{HttpHandler, MultiHandler};
use crate::proxy::forward::proxy::ProxyHandler;
use anyhow::Result;
use clap::Parser;
use hudsucker::rcgen::{CertificateParams, KeyPair};
use hudsucker::rustls::crypto::{aws_lc_rs};
use hudsucker::{certificate_authority::RcgenAuthority, Proxy};
use log::error;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::spawn;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}

#[derive(Parser, Debug)]
pub struct Args {
    /// path of the ca key
    #[arg(long, default_value = "ca.key")]
    ca_key: String,

    /// path of the ca cert
    #[arg(long, default_value = "ca.cert")]
    ca_cert: String,

    #[arg(long, default_value = "3000")]
    port: u16,
}

pub async fn run(args: &Args) -> Result<()> {
    let key_pair = fs::read_to_string(args.ca_key.clone()).expect("Failed to read CA key file");
    let ca_cert = fs::read_to_string(args.ca_cert.clone()).expect("Failed to read CA cert file");
    let key_pair = KeyPair::from_pem(key_pair.as_str()).expect("Failed to parse private key");
    let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert.as_str())
        .expect("Failed to parse CA certificate")
        .self_signed(&key_pair)
        .expect("Failed to sign CA certificate");

    spawn(async move {
        if let Err(error) = Store::global().watch().await {
            error!("Failed to watch pods: {}", error);
            std::process::exit(1);
        }
    });

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1_000, aws_lc_rs::default_provider());
    let handlers: Vec<Arc<dyn HttpHandler>> = vec![Arc::new(LogHandler), Arc::new(ProxyHandler)];
    let handler = MultiHandler::new(handlers);
    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([0, 0, 0, 0], args.port)))
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_http_handler(handler)
        .with_graceful_shutdown(shutdown_signal())
        .build()
        .expect("Failed to create proxy");

    if let Err(e) = proxy.start().await {
        error!("Failed to start proxy {}", e);
    }

    Ok(())
}
