use crate::cli::CLI;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Config {
    pub repo_url: String,
    pub clone_path: String,
    pub compose_file: String,
    pub mount_path: String,
    pub name: String,
    pub socket_path: String,
}

impl Config {
    pub fn from_env_and_cli(cli: &CLI) -> Result<Self, Box<dyn std::error::Error>> {
        // Parse env file into a HashMap (don't crash if it doesn't exist)
        let mut env_vars = HashMap::new();
        let env_path = &cli.env_file;
        if std::path::Path::new(env_path).exists() {
            if let Ok(env_content) = std::fs::read_to_string(env_path) {
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

        // Priority: CLI args > .env file > error/default
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

        let mount_path = cli
            .mount_path
            .clone()
            .or_else(|| env_vars.get("MOUNT_PATH").cloned())
            .ok_or(
                "MOUNT_PATH not provided. Use --mount-path flag or set MOUNT_PATH in .env file",
            )?;

        // New: allow compose_file, name, and socket_path from .env
        let compose_file = if cli.compose_file != "docker-compose.yml" {
            cli.compose_file.clone()
        } else {
            env_vars
                .get("COMPOSE_FILE")
                .cloned()
                .unwrap_or_else(|| cli.compose_file.clone())
        };

        let name = cli
            .name
            .clone()
            .or_else(|| env_vars.get("NAME").cloned())
            .ok_or("NAME not provided. Use --name flag or set NAME in .env file")?;
        let socket_path = if cli.socket_path != "/var/run/docker.sock" {
            cli.socket_path.clone()
        } else {
            env_vars
                .get("SOCKET_PATH")
                .cloned()
                .unwrap_or_else(|| cli.socket_path.clone())
        };

        Ok(Config {
            repo_url,
            clone_path,
            compose_file,
            mount_path,
            name,
            socket_path,
        })
    }

    pub fn show_configuration_help() {
        println!("Configuration options:");
        println!("  1. Command line flags:");
        println!(
            "     ./app v1.2.3 --name my-project --repo-url https://github.com/org/repo.git --mount-path /opt/configs --clone-path /opt/traefik-configs --compose-file ./docker-compose.yml --socket-path /var/run/docker.sock --env-file .env"
        );
        println!();
        println!("  2. Create a .env file (or use --env-file to specify a different file):");
        println!("     REPO_URL=https://github.com/your-org/traefik-config.git");
        println!("     CLONE_PATH=/opt/traefik-configs");
        println!("     MOUNT_PATH=/etc/traefik/dynamic");
        println!("     COMPOSE_FILE=./docker-compose.yml");
        println!("     NAME=my-project");
        println!("     SOCKET_PATH=/var/run/docker.sock");
        println!();
        println!("Command line flags take precedence over .env file values.");
    }
}
