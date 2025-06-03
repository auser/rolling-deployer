mod cli;
pub mod config;
pub mod deployment_manager;
pub mod docker_client;
pub mod git_client;
pub mod types;

use clap::Parser;
use cli::deploy as _deploy;
pub use cli::CLI;
use tracing_subscriber::EnvFilter;

pub async fn run() {
    let cli = CLI::parse();
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();
    _deploy(&cli).await;
}
