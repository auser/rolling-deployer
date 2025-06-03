use crate::cli::CLI;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    pub repo_url: String,
    pub clone_path: String,
    pub compose_file: String,
    pub mount_path_to_replace: Option<String>,
}

impl Config {
    pub fn from_env_and_cli(cli: &CLI) -> Result<Self, Box<dyn std::error::Error>> {
        // Parse .env file into a HashMap (don't crash if it doesn't exist)
        let mut env_vars = HashMap::new();
        if std::path::Path::new(".env").exists() {
            if let Ok(env_content) = std::fs::read_to_string(".env") {
                for line in env_content.lines() {
                    let line = line.trim();
                    // Skip empty lines and comments
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        env_vars.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        // Priority: CLI args > .env file > error
        let repo_url = cli
            .repo_url
            .clone()
            .or_else(|| env_vars.get("REPO_URL").cloned())
            .ok_or("REPO_URL not provided. Use --repo-url flag or set REPO_URL in .env file")?;

        let clone_path = cli
            .clone_path
            .clone()
            .or_else(|| env_vars.get("CLONE_PATH").cloned())
            .ok_or(
                "CLONE_PATH not provided. Use --clone-path flag or set CLONE_PATH in .env file",
            )?;

        Ok(Config {
            repo_url,
            clone_path,
            compose_file: cli.compose_file.clone(),
            mount_path_to_replace: cli.mount_path_to_replace.clone(),
        })
    }

    pub fn show_configuration_help() {
        println!("Configuration options:");
        println!("  1. Command line flags:");
        println!(
            "     ./app v1.2.3 --name my-project --repo-url https://github.com/org/repo.git --mount-path /opt/configs"
        );
        println!();
        println!("  2. Create a .env file:");
        println!("     REPO_URL=https://github.com/your-org/traefik-config.git");
        println!("     CLONE_PATH=/opt/traefik-configs");
        println!();
        println!("Command line flags take precedence over .env file values.");
    }
}
