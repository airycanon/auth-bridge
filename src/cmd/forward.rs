use crate::core::filter::AddressFilter;
use crate::core::pod::store::Store;
use crate::http::chain::Chain;
use crate::http::log::{log_request, log_response};
use crate::http::proxy::proxy_request;
use anyhow::Result;
use clap::Parser;
use hudsucker::rcgen::{CertificateParams, KeyPair};
use hudsucker::rustls::crypto::aws_lc_rs;
use hudsucker::{certificate_authority::RcgenAuthority, Proxy};
use log::error;
use rustls::crypto::ring;
use std::fs;
use std::net::SocketAddr;
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

    #[arg(long, default_value = "3149")]
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

    let chain = Chain::new()
        .with_request_handler(log_request)
        .with_request_handler(proxy_request::<_, AddressFilter>)
        .with_response_handler(log_response);

    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([0, 0, 0, 0], args.port)))
        .with_ca(ca)
        .with_rustls_client(ring::default_provider())
        .with_http_handler(chain)
        .with_graceful_shutdown(shutdown_signal())
        .build()
        .expect("Failed to create http");

    if let Err(e) = proxy.start().await {
        error!("Failed to start http {}", e);
    }

    Ok(())
}
