mod cli;
pub mod config;
pub mod deployment_manager;
pub mod docker_client;
pub mod git_client;
pub mod types;

use clap::Parser;
use cli::CLI;
pub use cli::deploy as _deploy;

pub async fn run() {
    let cli = CLI::parse();
    _deploy(&cli).await;
}
