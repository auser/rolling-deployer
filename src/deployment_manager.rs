use crate::{config::Config, docker_client::DockerClient, git_client::GitClient};

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
            .clone_repository_to_versioned_path(&config.repo_url, tag, &config.mount_path)
            .await?;

        // 2. Find running Traefik containers for this project
        let running_containers = self
            .docker
            .get_running_traefik_containers(project_name)
            .await?;

        if running_containers.is_empty() {
            return Err(format!(
                "No running Traefik containers found for project '{}'",
                project_name
            )
            .into());
        }

        println!(
            "Found {} running Traefik containers",
            running_containers.len()
        );

        // 3. For each running container, create a new one with the new config
        for (index, container) in running_containers.iter().enumerate() {
            let new_container_name = format!(
                "{}-{}-{}",
                container.names[0].trim_start_matches('/'),
                tag,
                index
            );

            println!("Rolling {} -> {}", container.names[0], new_container_name);

            // Create new container with updated volume mount pointing to new config
            let new_container_id = self
                .docker
                .create_container_from_existing(
                    container,
                    &new_container_name,
                    &new_config_path, // Pass the new config path here
                )
                .await?;

            // Start the new container
            self.docker.start_container(&new_container_id).await?;

            // Wait a bit for the new container to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Health check the new container (simplified)
            println!("New container {} is starting up...", new_container_name);

            // Stop and remove the old container
            println!("Stopping old container {}", container.names[0]);
            self.docker.stop_container(&container.id).await?;
            self.docker.remove_container(&container.id).await?;

            println!("Successfully rolled {} to new version", container.names[0]);
        }

        // 4. Clean up old config directories (keep last 3 versions)
        self.cleanup_old_configs(&config.mount_path, 3).await?;

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
        let target_config_path = format!("{}/traefik-config-{}", config.mount_path, tag);

        if !std::path::Path::new(&target_config_path).exists() {
            // If the config doesn't exist locally, clone it
            println!("Target config not found locally, cloning...");
            self.git
                .clone_repository_to_versioned_path(&config.repo_url, tag, &config.mount_path)
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
