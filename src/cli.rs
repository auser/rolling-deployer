use crate::config::Config;
use crate::deployment_manager::DeploymentManager;
use clap::Parser;

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
    #[arg(short, long, default_value = "/etc/traefik")]
    pub mount_path: Option<String>,
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity (-v, -vv, etc.)")]
    pub verbose: u8,
}

// Main application logic
pub async fn deploy(cli: &CLI) {
    // Load configuration from CLI args and/or .env file
    let config = match Config::from_env_and_cli(cli) {
        Ok(config) => {
            println!("Configuration loaded:");
            println!("  Repository: {}", config.repo_url);
            println!("  Mount path: {}", config.mount_path);
            config
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            println!();
            Config::show_configuration_help();
            return;
        }
    };

    let deployment_manager = DeploymentManager::new(cli.socket_path.clone());

    if let Some(project_name) = &cli.name {
        println!(
            "Starting deployment for project '{}' with tag '{}'",
            project_name, cli.tag
        );

        match deployment_manager
            .rolling_deploy(project_name, &cli.tag, &config)
            .await
        {
            Ok(()) => println!("Rolling deployment successful!"),
            Err(e) => eprintln!("Rolling deployment failed: {}", e),
        }
    } else {
        eprintln!("Project name is required for deployment. Use --name flag.");
        println!();
        println!("Examples:");
        println!(
            "  ./app v1.2.3 --name my-traefik-project --repo-url https://github.com/org/repo.git --mount-path /opt/configs"
        );
        println!(
            "  ./app v1.2.3 --name my-traefik-project  # Uses .env file for repo and mount path"
        );
    }
}
