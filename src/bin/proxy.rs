use std::fs;
use hudsucker::{
    certificate_authority::RcgenAuthority,
    rcgen::{CertificateParams, KeyPair},
    *,
};
use std::net::SocketAddr;
use clap::Parser;
use log::error;
use auth_bridge::handlers::multi::{HandlerEnum, MultiHandler};

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path of the ca key
    #[arg(long, default_value = "ca.key")]
    ca_key: String,

    /// path of the ca cert
    #[arg(long, default_value = "ca.cert")]
    ca_cert: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let key_pair = fs::read_to_string(args.ca_key)
        .expect("Failed to read CA key file");
    let ca_cert = fs::read_to_string(args.ca_cert)
        .expect("Failed to read CA cert file");
    let key_pair = KeyPair::from_pem(key_pair.as_str()).expect("Failed to parse private key");
    let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert.as_str())
        .expect("Failed to parse CA certificate")
        .self_signed(&key_pair)
        .expect("Failed to sign CA certificate");

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1_000);

    let handlers = vec!(
        HandlerEnum::Log,
        HandlerEnum::Policy
    );
    let handler = MultiHandler::new(handlers);

    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 7749)))
        .with_rustls_client()
        .with_ca(ca)
        .with_http_handler(handler)
        .with_graceful_shutdown(shutdown_signal())
        .build();

    if let Err(e) = proxy.start().await {
        error!("{}", e);
    }
}
