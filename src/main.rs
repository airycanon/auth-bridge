use anyhow::Result;
use auth_bridge::apis::proxy::Proxy;
use auth_bridge::apis::script::Script;
use auth_bridge::cmd::controller;
use auth_bridge::cmd::forward;
use auth_bridge::cmd::forward::Args as ForwardArgs;
use auth_bridge::cmd::reverse;
use auth_bridge::cmd::reverse::Args as ReverseArgs;
use clap::{Parser, Subcommand};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use rustls::crypto::ring;
use kube::CustomResourceExt;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(long_about = "run controller")]
    Controller,
    #[command(long_about = "generate crd")]
    Crd {
        #[arg(long)]
        path: String,
    },
    #[command(long_about = "run forward http")]
    ForwardProxy(ForwardArgs),
    #[command(long_about = "run reverse http")]
    ReverseProxy(ReverseArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::ForwardProxy(args) => forward::run(args).await,
        Commands::ReverseProxy(args) => reverse::run(args).await,
        Commands::Controller => controller::run().await,
        Commands::Crd { path } => {
            generate_crd(Proxy::crd(), path)?;
            generate_crd(Script::crd(), path)?;

            Ok(())
        }
    }
}

fn generate_crd(crd: CustomResourceDefinition, path: &String) -> Result<()> {
    let yaml = serde_yaml::to_string(&crd)?;
    let file = format!("{}_{}.yaml", crd.spec.group, crd.spec.names.plural);
    std::fs::write(format!("{}/{}", path, file), yaml)?;

    Ok(())
}
