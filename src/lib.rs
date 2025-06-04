pub mod cli;
pub mod config;
pub mod deployment_manager;
pub mod docker_client;
pub mod git_client;
pub mod types;

use clap::Parser;
use cli::deploy as _deploy;
pub use cli::CLI;

pub async fn run() {
    let cli = CLI::parse();
    _deploy(cli).await;
}
