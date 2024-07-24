use std::fs;
use hudsucker::{certificate_authority::RcgenAuthority, Proxy};
use hudsucker::rcgen::{CertificateParams, KeyPair};
use std::net::SocketAddr;
use tokio::sync::Mutex;
use clap::Parser;
use k8s_openapi::api::core::v1::Pod;
use kube::{Client, Api};
use log::{error};
use tokio::spawn;
use crate::handlers::multi::{HandlerEnum, MultiHandler};
use lazy_static::lazy_static;
use futures::TryStreamExt;
use kube::runtime::{watcher, watcher::Error};
use anyhow::Result;
use crate::apis::pod_meta;

lazy_static! {
    static ref RESOURCE_VERSION: Mutex<String> = Mutex::new(String::new());
}

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
}

pub async fn run(args: &Args) -> Result<()> {
    let key_pair = fs::read_to_string(args.ca_key.clone())
        .expect("Failed to read CA key file");
    let ca_cert = fs::read_to_string(args.ca_cert.clone())
        .expect("Failed to read CA cert file");
    let key_pair = KeyPair::from_pem(key_pair.as_str()).expect("Failed to parse private key");
    let ca_cert = CertificateParams::from_ca_cert_pem(ca_cert.as_str())
        .expect("Failed to parse CA certificate")
        .self_signed(&key_pair)
        .expect("Failed to sign CA certificate");

    spawn(async move {
        if let Err(error) = watch_pods().await {
            error!("Failed to watch pods: {}", error);
            std::process::exit(1);
        }
    });

    let ca = RcgenAuthority::new(key_pair, ca_cert, 1_000);
    let handlers = vec!(
        HandlerEnum::Log,
        HandlerEnum::Policy
    );
    let handler = MultiHandler::new(handlers);
    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([0, 0, 0, 0], 7749)))
        .with_rustls_client()
        .with_ca(ca)
        .with_http_handler(handler)
        .with_graceful_shutdown(shutdown_signal())
        .build();

    if let Err(e) = proxy.start().await {
        error!("Failed to start proxy {}", e);
    }

    Ok(())
}

async fn watch_pods() -> Result<(), Error> {
    let client = Client::try_default().await.expect("Failed to init k8s client");
    let api = Api::<Pod>::all(client);
    let watcher = watcher(api, watcher::Config::default());

    watcher.try_for_each(|event| async {
        match event {
            watcher::Event::Applied(pod) => pod_meta::bind(&pod),
            watcher::Event::Deleted(pod) => pod_meta::unbind(&pod),
            watcher::Event::Restarted(pods) => pod_meta::bind_all(pods),
        }
        Ok(())
    }).await
}

