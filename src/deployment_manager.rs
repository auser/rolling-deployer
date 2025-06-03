use crate::{config::Config, docker_client::DockerClient, git_client::GitClient};
use serde_yaml::Value;
use std::path::Path;

pub struct DeploymentManager {
    docker: DockerClient,
    git: GitClient,
}

impl DeploymentManager {
    pub fn new(socket_path: String) -> Self {
        Self {
            docker: DockerClient::new(socket_path),
            git: GitClient,
        }
    }

    fn update_compose_file_volume_source(
        compose_file: &str,
        new_config_path: &str,
        mount_path_to_replace: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(compose_file)?;
        let mut doc: Value = serde_yaml::from_str(&content)?;
        let mut replaced = false;

        if let Some(services) = doc.get_mut("services").and_then(Value::as_mapping_mut) {
            for (_svc_name, svc) in services.iter_mut() {
                if let Some(vols) = svc.get_mut("volumes").and_then(Value::as_sequence_mut) {
                    for vol in vols.iter_mut() {
                        // Handle string form: "host:container[:mode]"
                        if let Some(s) = vol.as_str() {
                            let parts: Vec<&str> = s.split(':').collect();
                            if parts.len() >= 2 {
                                let host = parts[0];
                                let should_replace = if let Some(user_path) = mount_path_to_replace
                                {
                                    host == user_path
                                } else {
                                    host.ends_with("/current")
                                };
                                if should_replace && !replaced {
                                    let mut new_vol = new_config_path.to_string();
                                    for p in &parts[1..] {
                                        new_vol.push(':');
                                        new_vol.push_str(p);
                                    }
                                    *vol = Value::String(new_vol);
                                    replaced = true;
                                }
                            }
                        }
                        // Handle map form (YAML 1.2): {type: bind, source: ..., target: ...}
                        else if let Some(map) = vol.as_mapping_mut() {
                            if let Some(source) = map
                                .get(&Value::String("source".to_string()))
                                .and_then(Value::as_str)
                            {
                                let should_replace = if let Some(user_path) = mount_path_to_replace
                                {
                                    source == user_path
                                } else {
                                    source.ends_with("/current")
                                };
                                if should_replace && !replaced {
                                    map.insert(
                                        Value::String("source".to_string()),
                                        Value::String(new_config_path.to_string()),
                                    );
                                    replaced = true;
                                }
                            }
                        }
                        if replaced {
                            break;
                        }
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
        project_name: &str,
        tag: &str,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Starting rolling deployment for project '{}' with tag '{}'",
            project_name, tag
        );

        // 1. Clone the new configuration to a versioned directory
        let new_config_path = self
            .git
            .clone_repository_to_versioned_path(&config.repo_url, tag, &config.clone_path)
            .await?;

        // 1.5. Update the compose file to use the new config path as the volume source
        // NOTE: You must add serde_yaml = "*" to Cargo.toml
        Self::update_compose_file_volume_source(
            &config.compose_file,
            &new_config_path,
            &config.mount_path_to_replace,
        )?;

        // 2. Find running Traefik containers for this project
        let running_containers = self
            .docker
            .get_running_containers_by_image_substring(project_name)
            .await?;

        if running_containers.is_empty() {
            return Err(
                format!("No running containers found for project '{}'", project_name).into(),
            );
        }

        println!(
            "Found {} running Traefik containers",
            running_containers.len()
        );

        // 3. For each running container, recreate the service
        for (_index, container) in running_containers.iter().enumerate() {
            let service_name = container.names[0].trim_start_matches('/');
            println!("Rolling service: {}", service_name);

            // Determine the directory containing the compose file
            let compose_dir = Path::new(&config.compose_file)
                .parent()
                .unwrap_or_else(|| Path::new("."));

            // Run docker compose up -d --force-recreate <service> in the compose file's directory
            let status = std::process::Command::new("docker")
                .args([
                    "compose",
                    "-f",
                    &config.compose_file,
                    "up",
                    "-d",
                    "--force-recreate",
                    service_name,
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
        self.rolling_deploy(project_name, tag, config).await?;

        println!("Rollback completed successfully!");
        Ok(())
    }
}
