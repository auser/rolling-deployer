use tracing::info;

pub struct GitClient;

impl GitClient {
    pub async fn clone_repository_to_versioned_path(
        &self,
        repo_url: &str,
        tag: &str,
        base_path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let versioned_path = format!("{}/traefik-config-{}", base_path, tag);
        let symlink_path = format!("{}/current", base_path);

        info!(
            "Cloning repository {} at tag {} to {}",
            repo_url, tag, versioned_path
        );

        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&versioned_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Only clone if the versioned directory does not exist
        if !std::path::Path::new(&versioned_path).exists() {
            // Clone the repository
            let output = std::process::Command::new("git")
                .args(&[
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    tag,
                    repo_url,
                    &versioned_path,
                ])
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "Git clone failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            info!(
                "Successfully cloned {} at tag {} to {}",
                repo_url, tag, versioned_path
            );
        } else {
            info!("Using existing config at {}", versioned_path);
        }

        // Create or update the 'current' symlink
        let symlink_path_obj = std::path::Path::new(&symlink_path);
        if symlink_path_obj.exists() || symlink_path_obj.is_symlink() {
            std::fs::remove_file(&symlink_path)?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&versioned_path, &symlink_path)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&versioned_path, &symlink_path)?;

        Ok(symlink_path)
    }

    pub async fn fetch_latest(&self, repo_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Fetching latest changes in {}", repo_dir);

        let output = std::process::Command::new("git")
            .args(&["fetch", "--all"])
            .current_dir(repo_dir)
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Git fetch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(())
    }

    pub async fn checkout_tag(
        &self,
        repo_dir: &str,
        tag: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Checking out tag {} in {}", tag, repo_dir);

        let output = std::process::Command::new("git")
            .args(&["checkout", tag])
            .current_dir(repo_dir)
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        info!("Successfully checked out tag {}", tag);
        Ok(())
    }
}
