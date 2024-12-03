use anyhow::Result;
use auth_bridge::cmd::controller;
use auth_bridge::cmd::forward;
use auth_bridge::cmd::forward::Args as ForwardArgs;
use auth_bridge::cmd::reverse;
use auth_bridge::cmd::reverse::Args as ReverseArgs;
use clap::{Parser, Subcommand};

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
    #[command(long_about = "run forward proxy")]
    ForwardProxy(ForwardArgs),
    #[command(long_about = "run reverse proxy")]
    ReverseProxy(ReverseArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::ForwardProxy(args) => forward::run(args).await,
        Commands::ReverseProxy(args) => reverse::run(args).await,
        Commands::Controller => controller::run().await,
    }
}
