use crate::{config::Config, docker_client::DockerClient, git_client::GitClient};
use serde_yaml::Value;
use std::path::Path;

pub struct DeploymentManager {
    docker: DockerClient,
    git: GitClient,
    config: Config,
}

impl DeploymentManager {
    pub fn new(config: Config) -> Self {
        Self {
            docker: DockerClient::new(config.socket_path.clone()),
            git: GitClient,
            config,
        }
    }

    /// Robustly extract the service name from a container.
    /// Prefer the Docker Compose label if present, otherwise parse the container name.
    fn extract_service_name(container: &crate::types::Container) -> String {
        // Try Docker Compose label first
        if let Some(labels) = &container.labels {
            if let Some(service) = labels.get("com.docker.compose.service") {
                return service.clone();
            }
        }
        // Fallback: parse from container name
        if let Some(name) = container.names.get(0) {
            let name = name.trim_start_matches('/');
            // Try underscore split (compose v2 default: <project>_<service>_<index>)
            let underscore_parts: Vec<&str> = name.split('_').collect();
            if underscore_parts.len() >= 3 {
                return underscore_parts[underscore_parts.len() - 2].to_string();
            }
            // Try dash split (older compose: <something>-<service>-<index>)
            let dash_parts: Vec<&str> = name.split('-').collect();
            if dash_parts.len() >= 2 {
                // If last part is a number, use the one before it
                if dash_parts.last().unwrap().parse::<u32>().is_ok() {
                    return dash_parts[dash_parts.len() - 2].to_string();
                }
            }
            // Fallback: just return the name
            return name.to_string();
        }
        // If all else fails, empty string
        String::new()
    }

    fn update_compose_file_volume_source(
        compose_file: &str,
        symlink_path: &str,
        mount_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(compose_file)?;
        let mut doc: Value = serde_yaml::from_str(&content)?;
        let mut replaced = false;

        if let Some(services) = doc.get_mut("services").and_then(Value::as_mapping_mut) {
            for (_svc_name, svc) in services.iter_mut() {
                if let Some(vols) = svc.get_mut("volumes").and_then(Value::as_sequence_mut) {
                    // Try to find and update an existing mapping
                    for vol in vols.iter_mut() {
                        // Handle string form: "host:container[:mode]"
                        if let Some(s) = vol.as_str() {
                            let parts: Vec<&str> = s.split(':').collect();
                            if parts.len() >= 2 {
                                let target = parts[1];
                                if target == mount_path && !replaced {
                                    // preserve mode if present
                                    let mut new_vol = format!("{}:{}", symlink_path, mount_path);
                                    if parts.len() > 2 {
                                        new_vol.push(':');
                                        new_vol.push_str(parts[2]);
                                    }
                                    *vol = Value::String(new_vol);
                                    replaced = true;
                                }
                            }
                        }
                        // Handle map form (YAML 1.2): {type: bind, source: ..., target: ...}
                        else if let Some(map) = vol.as_mapping_mut() {
                            if let Some(target) = map
                                .get(&Value::String("target".to_string()))
                                .and_then(Value::as_str)
                            {
                                if target == mount_path && !replaced {
                                    map.insert(
                                        Value::String("source".to_string()),
                                        Value::String(symlink_path.to_string()),
                                    );
                                    replaced = true;
                                }
                            }
                        }
                        if replaced {
                            break;
                        }
                    }
                    // If not found, add a new mapping
                    if !replaced {
                        // Default to rw mode
                        let new_vol = format!("{}:{}:rw", symlink_path, mount_path);
                        vols.push(Value::String(new_vol));
                        replaced = true;
                    }
                }
                if replaced {
                    break;
                }
            }
        }
        if replaced {
            let updated = serde_yaml::to_string(&doc)?;
            std::fs::write(compose_file, updated)?;
        }
        Ok(())
    }

    pub async fn rolling_deploy(
        &self,
        tag: &str,
        swarm: bool,
        swarm_service: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = &self.config;
        println!(
            "Starting rolling deployment for project '{}' with tag '{}'",
            config.name, tag
        );

        // 1. Clone the new configuration to a versioned directory
        let symlink_path = self
            .git
            .clone_repository_to_versioned_path(&config.repo_url, tag, &config.clone_path)
            .await?;

        // 1.5. Update the compose file to use the new config path as the volume source
        // NOTE: You must add serde_yaml = "*" to Cargo.toml
        Self::update_compose_file_volume_source(
            &config.compose_file,
            &symlink_path,
            &config.mount_path,
        )?;

        if swarm {
            let service = match swarm_service {
                Some(ref s) if !s.is_empty() => s,
                _ => {
                    return Err(
                        "In swarm mode, --swarm-service or SWARM_SERVICE must be specified".into(),
                    );
                }
            };
            println!(
                "Swarm mode: updating service '{}' mount to new config path.",
                service
            );
            // Remove the old mount and add the new one
            let _remove_arg = format!(
                "type=bind,src={},dst={}",
                config.clone_path, config.mount_path
            );
            let add_arg = format!("type=bind,src={},dst={}", symlink_path, config.mount_path);
            let status = std::process::Command::new("docker")
                .args([
                    "service",
                    "update",
                    "--mount-rm",
                    &config.mount_path,
                    "--mount-add",
                    &add_arg,
                    service,
                ])
                .status()?;
            if !status.success() {
                return Err(format!("docker service update failed for service {}", service).into());
            }
            println!("Successfully updated service '{}' in Swarm mode.", service);
        } else {
            // 2. Find running Traefik containers for this project
            let running_containers = self
                .docker
                .get_running_containers_by_image_substring(&config.name)
                .await?;

            if running_containers.is_empty() {
                return Err(
                    format!("No running containers found for project '{}'", config.name).into(),
                );
            }

            println!(
                "Found {} running Traefik containers",
                running_containers.len()
            );

            // 3. For each running container, recreate the service
            for (_index, container) in running_containers.iter().enumerate() {
                let service_name = Self::extract_service_name(container);
                println!("Rolling service: {}", service_name);

                // Determine the absolute path to the compose file
                let compose_file_abs = std::fs::canonicalize(&config.compose_file)?;
                let compose_dir = compose_file_abs.parent().unwrap_or_else(|| Path::new("."));

                // Check if the directory exists
                if !compose_dir.exists() {
                    return Err(format!(
                        "Compose directory does not exist: {}",
                        compose_dir.display()
                    )
                    .into());
                }

                // Run docker compose up -d --force-recreate <service> in the compose file's directory
                let status = std::process::Command::new("docker")
                    .args([
                        "compose",
                        "-f",
                        compose_file_abs.to_str().unwrap(),
                        "up",
                        "-d",
                        "--force-recreate",
                        service_name.as_str(),
                    ])
                    .current_dir(compose_dir)
                    .status()?;

                if !status.success() {
                    return Err(
                        format!("docker compose up failed for service {}", service_name).into(),
                    );
                }

                println!("Successfully rolled {} to new version", service_name);
            }
        }

        // 4. Clean up old config directories (keep last 3 versions)
        self.cleanup_old_configs(&config.clone_path, 3).await?;

        println!("Rolling deployment completed successfully!");
        Ok(())
    }

    async fn cleanup_old_configs(
        &self,
        base_path: &str,
        keep_versions: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut config_dirs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("traefik-config-") {
                                config_dirs.push(path);
                            }
                        }
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        config_dirs.sort_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|m| m.created())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        config_dirs.reverse();

        // Remove old versions beyond the keep limit
        for old_config in config_dirs.iter().skip(keep_versions) {
            println!("Cleaning up old config: {:?}", old_config);
            if let Err(e) = std::fs::remove_dir_all(old_config) {
                eprintln!("Failed to remove old config {:?}: {}", old_config, e);
            }
        }

        Ok(())
    }

    pub async fn rollback(
        &self,
        project_name: &str,
        tag: &str,
        config: &Config,
        swarm: bool,
        swarm_service: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Starting rollback of project '{}' to tag '{}'",
            project_name, tag
        );

        // Check if the target version already exists
        let target_config_path = format!("{}/traefik-config-{}", config.clone_path, tag);

        if !std::path::Path::new(&target_config_path).exists() {
            // If the config doesn't exist locally, clone it
            println!("Target config not found locally, cloning...");
            self.git
                .clone_repository_to_versioned_path(&config.repo_url, tag, &config.clone_path)
                .await?;
        } else {
            println!("Using existing config at {}", target_config_path);
        }

        // Perform rolling deployment to the target tag
        self.rolling_deploy(tag, swarm, swarm_service).await?;

        println!("Rollback completed successfully!");
        Ok(())
    }
}
