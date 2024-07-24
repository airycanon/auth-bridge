use clap::{Parser, Subcommand};
use auth_bridge::cmd::{proxy,controller};
use anyhow::Result;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Controller,
    Proxy(proxy::Args)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Proxy(args) => {
            proxy::run(args).await
        },
        Commands::Controller => {
            controller::run().await
        }
    }
}