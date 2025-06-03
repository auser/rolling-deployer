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

        info!(
            "Cloning repository {} at tag {} to {}",
            repo_url, tag, versioned_path
        );

        // Remove existing directory if it exists
        if std::path::Path::new(&versioned_path).exists() {
            std::fs::remove_dir_all(&versioned_path)?;
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&versioned_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

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
        Ok(versioned_path)
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
