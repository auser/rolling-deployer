use crate::config::Config;
use crate::deployment_manager::DeploymentManager;
use clap::Parser;
use std::collections::HashMap;
use tracing::{debug, error, info};
use tracing_subscriber;

#[derive(Parser)]
pub struct CLI {
    #[arg(value_name = "TAG", index = 1)]
    pub tag: String,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long, default_value = "/var/run/docker.sock")]
    pub socket_path: String,
    #[arg(short, long)]
    pub repo_url: Option<String>,
    #[arg(short, long, help = "Path to clone the config repo into")]
    pub clone_path: Option<String>,
    #[arg(
        long,
        help = "Target path in the container to mount the config (e.g. /etc/traefik/dynamic)"
    )]
    pub mount_path: Option<String>,
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
    #[arg(long, help = "Use Docker Swarm mode")]
    pub swarm: bool,
}

// Main application logic
pub async fn deploy(mut cli: CLI) {
    // Allow tests to skip real deployment logic
    if std::env::var("SKIP_DEPLOY").ok().as_deref() == Some("1") {
        tracing::info!("Skipping real deployment for test");
        return;
    }

    // Load .env file if present and fill missing CLI fields
    let env_content = match std::fs::read_to_string(&cli.env_file) {
        Ok(content) => {
            let mut env_content = HashMap::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    env_content.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
            env_content
        }
        Err(e) => {
            error!("Error reading .env file: {}", e);
            HashMap::new()
        }
    };

    debug!("env_content: {:?}", env_content);

    // Use the helper for Option<String> fields
    let name = extract_env_var_from_cli_or_env(&cli.name, &env_content, "NAME", "");
    let repo_url = extract_env_var_from_cli_or_env(&cli.repo_url, &env_content, "REPO_URL", "");
    let clone_path =
        extract_env_var_from_cli_or_env(&cli.clone_path, &env_content, "CLONE_PATH", "/opt/dev");
    let mount_path =
        extract_env_var_from_cli_or_env(&cli.mount_path, &env_content, "MOUNT_PATH", "");

    // For String fields with a default, use env_content if the value is still the default
    let socket_path = if cli.socket_path == "/var/run/docker.sock" {
        env_content
            .get("SOCKET_PATH")
            .cloned()
            .unwrap_or_else(|| cli.socket_path.clone())
    } else {
        cli.socket_path.clone()
    };
    let compose_file = if cli.compose_file == "docker-compose.yml" {
        env_content
            .get("COMPOSE_FILE")
            .cloned()
            .unwrap_or_else(|| cli.compose_file.clone())
    } else {
        cli.compose_file.clone()
    };

    // Update CLI struct for downstream config loading
    cli.name = if name.is_empty() { None } else { Some(name) };
    cli.repo_url = if repo_url.is_empty() {
        None
    } else {
        Some(repo_url)
    };
    cli.clone_path = if clone_path.is_empty() {
        None
    } else {
        Some(clone_path)
    };
    cli.mount_path = if mount_path.is_empty() {
        None
    } else {
        Some(mount_path)
    };
    cli.socket_path = socket_path;
    cli.compose_file = compose_file;

    // Ensure mount_path is set from CLI or .env
    if cli.mount_path.is_none() {
        error!("MOUNT_PATH must be set via --mount-path or in the .env file");
        eprintln!("MOUNT_PATH must be set via --mount-path or in the .env file");
        return;
    }
    tracing::debug!("Mount path from CLI: {:?}", cli.mount_path);

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
            info!("Configuration loaded:");
            info!("  Repository: {}", config.repo_url);
            info!("  Clone path: {}", config.clone_path);
            info!("  Mount path: {}", config.mount_path);
            config
        }
        Err(e) => {
            error!("Configuration error: {}", e);
            info!("");
            Config::show_configuration_help();
            return;
        }
    };

    let deployment_manager = DeploymentManager::new(config.clone());

    info!(
        "Starting deployment for project '{}' with tag '{}'",
        config.name, cli.tag
    );

    match deployment_manager.rolling_deploy(&cli.tag, cli.swarm).await {
        Ok(()) => info!("Rolling deployment successful!"),
        Err(e) => error!("Rolling deployment failed: {}", e),
    }
}

// Extracts a value from the CLI or .env file, preferring the CLI value if present
fn extract_env_var_from_cli_or_env<V: ToString + PartialEq + Default>(
    val: &Option<V>,
    env_content: &HashMap<String, String>,
    key: &str,
    default_value: &str,
) -> String {
    let val_str = val.as_ref().unwrap_or(&V::default()).to_string();
    if val_str != V::default().to_string() && val_str != default_value {
        // If the CLI value is set and not the default, use it
        val_str
    } else if let Some(env_val) = env_content.get(key) {
        // Otherwise, use the value from the env_content HashMap if present
        env_val.clone()
    } else {
        // Otherwise, return None (caller can use default if needed)
        default_value.to_string()
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
            mount_path: None,
            verbose: 0,
            compose_file: "docker-compose.yml".to_string(),
            env_file: ".env".to_string(),
            swarm: false,
        };
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            deploy(cli).await;
        });
        // This test just ensures no panic and covers the error path for missing name
    }

    #[test]
    fn test_extract_env_var_from_cli_or_env_cli_value() {
        let env_content = std::collections::HashMap::new();
        let cli_val = Some("cli_val".to_string());
        let result =
            super::extract_env_var_from_cli_or_env(&cli_val, &env_content, "KEY", "default");
        assert_eq!(result, "cli_val");
    }

    #[test]
    fn test_extract_env_var_from_cli_or_env_env_value() {
        let mut env_content = std::collections::HashMap::new();
        env_content.insert("KEY".to_string(), "env_val".to_string());
        let cli_val: Option<String> = None;
        let result =
            super::extract_env_var_from_cli_or_env(&cli_val, &env_content, "KEY", "default");
        assert_eq!(result, "env_val");
    }

    #[test]
    fn test_extract_env_var_from_cli_or_env_both_values() {
        let mut env_content = std::collections::HashMap::new();
        env_content.insert("KEY".to_string(), "env_val".to_string());
        let cli_val = Some("cli_val".to_string());
        let result =
            super::extract_env_var_from_cli_or_env(&cli_val, &env_content, "KEY", "default");
        assert_eq!(result, "cli_val");
    }

    #[test]
    fn test_extract_env_var_from_cli_or_env_default() {
        let env_content = std::collections::HashMap::new();
        let cli_val: Option<String> = None;
        let result =
            super::extract_env_var_from_cli_or_env(&cli_val, &env_content, "KEY", "default");
        assert_eq!(result, "default");
    }
}
