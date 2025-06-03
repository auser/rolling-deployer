use crate::config::Config;
use crate::deployment_manager::DeploymentManager;
use clap::Parser;
use tracing_subscriber;

#[derive(Parser)]
pub struct CLI {
    #[arg(value_name = "TAG", index = 1)]
    pub tag: String,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long, default_value = "/var/run/docker.sock")]
    pub socket_path: String,
    #[arg(
        short,
        long,
        default_value = "https://bitbucket.org:financialpayments/plain-jane-proxy.git"
    )]
    pub repo_url: Option<String>,
    #[arg(
        short,
        long,
        default_value = "/etc/traefik",
        help = "Path to clone the config repo into"
    )]
    pub clone_path: Option<String>,
    #[arg(
        long,
        help = "Target path in the container to mount the config (e.g. /etc/traefik/dynamic)",
        required = true
    )]
    pub mount_path: String,
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity (-v, -vv, etc.)")]
    pub verbose: u8,
    #[arg(long, default_value = "docker-compose.yml")]
    pub compose_file: String,
    #[arg(
        short = 'e',
        long = "env-file",
        default_value = ".env",
        help = "Path to .env file"
    )]
    pub env_file: String,
}

// Main application logic
pub async fn deploy(mut cli: CLI) {
    // Load .env file if present and fill missing CLI fields
    let env_path = &cli.env_file;
    if std::path::Path::new(env_path).exists() {
        if let Ok(env_content) = std::fs::read_to_string(env_path) {
            for line in env_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if cli.name.is_none() && key == "NAME" {
                        cli.name = Some(value.to_string());
                    }
                    if cli.socket_path == "/var/run/docker.sock" && key == "SOCKET_PATH" {
                        cli.socket_path = value.to_string();
                    }
                }
            }
        }
    }
    // Load configuration from CLI args and/or .env file
    let config = match Config::from_env_and_cli(&cli) {
        Ok(config) => {
            // Set up logging based on config.verbose
            let filter = match cli.verbose {
                0 => "warn",
                1 => "info",
                2 => "debug",
                _ => "trace",
            };
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
                .init();
            println!("Configuration loaded:");
            println!("  Repository: {}", config.repo_url);
            println!("  Mount path: {}", config.clone_path);
            config
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            println!();
            Config::show_configuration_help();
            return;
        }
    };

    let deployment_manager = DeploymentManager::new(config.clone());

    println!(
        "Starting deployment for project '{}' with tag '{}'",
        config.name, cli.tag
    );

    match deployment_manager.rolling_deploy(&cli.tag).await {
        Ok(()) => println!("Rolling deployment successful!"),
        Err(e) => eprintln!("Rolling deployment failed: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tokio::runtime::Runtime;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            // Set up any global state, env vars, etc. if needed
        });
    }

    #[test]
    fn test_deploy_missing_name() {
        setup();
        let cli = CLI {
            tag: "v1.0.0".to_string(),
            name: None,
            socket_path: "/tmp/docker.sock".to_string(),
            repo_url: Some("https://example.com/repo.git".to_string()),
            clone_path: Some("/tmp/mount".to_string()),
            mount_path: String::new(),
            verbose: 0,
            compose_file: "docker-compose.yml".to_string(),
            env_file: ".env".to_string(),
        };
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            deploy(cli).await;
        });
        // This test just ensures no panic and covers the error path for missing name
    }
}
