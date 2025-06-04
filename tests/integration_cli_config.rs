use rolling_deployer::cli::{deploy, CLI};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

const TEST_REPO_URL: &str = "https://github.com/auser/empty-repo-for-testing.git";

#[tokio::test]
async fn test_cli_precedence_over_env() {
    std::env::set_var("SKIP_DEPLOY", "1");
    let temp_dir = tempdir().unwrap();
    let clone_path = temp_dir.path().join("cli_clone");
    let mount_path = temp_dir.path().join("cli_mount");
    let env_path = temp_dir.path().join("test_cli_precedence.env");
    let compose_file_path = temp_dir.path().join("cli-compose.yml");
    let mut file = File::create(&env_path).unwrap();
    writeln!(file, "NAME=env_name").unwrap();
    writeln!(file, "REPO_URL={}", TEST_REPO_URL).unwrap();
    writeln!(file, "CLONE_PATH={}", clone_path.display()).unwrap();
    writeln!(file, "MOUNT_PATH={}", mount_path.display()).unwrap();
    writeln!(
        file,
        "SOCKET_PATH={}/env_socket.sock",
        temp_dir.path().display()
    )
    .unwrap();
    writeln!(file, "COMPOSE_FILE={}", compose_file_path.display()).unwrap();

    // Create the compose file expected by the test
    File::create(&compose_file_path).unwrap();

    let cli = CLI {
        tag: "v1.2.3".to_string(),
        name: Some("cli_name".to_string()),
        socket_path: temp_dir
            .path()
            .join("cli_socket.sock")
            .display()
            .to_string(),
        repo_url: Some(TEST_REPO_URL.to_string()),
        clone_path: Some(clone_path.display().to_string()),
        mount_path: Some(mount_path.display().to_string()),
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: false,
        swarm_service: None,
    };

    deploy(cli).await;
}

#[tokio::test]
async fn test_env_used_when_cli_missing() {
    std::env::set_var("SKIP_DEPLOY", "1");
    let temp_dir = tempdir().unwrap();
    let clone_path = temp_dir.path().join("env_clone");
    let mount_path = temp_dir.path().join("env_mount");
    let env_path = temp_dir.path().join("test_env_used.env");
    let compose_file_path = temp_dir.path().join("env-compose.yml");
    let mut file = File::create(&env_path).unwrap();
    writeln!(file, "NAME=env_name").unwrap();
    writeln!(file, "REPO_URL={}", TEST_REPO_URL).unwrap();
    writeln!(file, "CLONE_PATH={}", clone_path.display()).unwrap();
    writeln!(file, "MOUNT_PATH={}", mount_path.display()).unwrap();
    writeln!(
        file,
        "SOCKET_PATH={}/env_socket.sock",
        temp_dir.path().display()
    )
    .unwrap();
    writeln!(file, "COMPOSE_FILE={}", compose_file_path.display()).unwrap();

    // Ensure clone_path and mount_path directories exist
    std::fs::create_dir_all(&clone_path).unwrap();
    std::fs::create_dir_all(&mount_path).unwrap();

    // Create the compose file in the expected cloned repo directory
    let versioned_clone_path = clone_path.join("traefik-config-v1.2.3");
    std::fs::create_dir_all(&versioned_clone_path).unwrap();
    let compose_file_in_repo = versioned_clone_path.join("env-compose.yml");
    File::create(&compose_file_in_repo).unwrap();

    // Also create the compose file at the path specified in the env (for completeness)
    File::create(&compose_file_path).unwrap();

    let cli = CLI {
        tag: "v1.2.3".to_string(),
        name: None,
        socket_path: "/var/run/docker.sock".to_string(),
        repo_url: None,
        clone_path: None,
        mount_path: None,
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: false,
        swarm_service: None,
    };

    deploy(cli).await;
}

#[tokio::test]
async fn test_default_used_when_none_set() {
    std::env::set_var("SKIP_DEPLOY", "1");
    let temp_dir = tempdir().unwrap();
    let mount_path = temp_dir.path().join("default_mount");
    let env_path = temp_dir.path().join("test_default.env");
    let compose_file_path = temp_dir.path().join("docker-compose.yml");
    File::create(&env_path).unwrap(); // empty .env
    File::create(&compose_file_path).unwrap();

    let cli = CLI {
        tag: "v1.2.3".to_string(),
        name: None,
        socket_path: "/var/run/docker.sock".to_string(),
        repo_url: Some(TEST_REPO_URL.to_string()),
        clone_path: None,
        mount_path: Some(mount_path.display().to_string()), // required
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: false,
        swarm_service: None,
    };

    deploy(cli).await;
}

#[tokio::test]
async fn test_swarm_mode_flag() {
    std::env::set_var("SKIP_DEPLOY", "1");
    let temp_dir = tempdir().unwrap();
    let clone_path = temp_dir.path().join("swarm_clone");
    let mount_path = temp_dir.path().join("swarm_mount");
    let env_path = temp_dir.path().join("test_swarm.env");
    let compose_file_path = temp_dir.path().join("swarm-compose.yml");
    let mut file = File::create(&env_path).unwrap();
    writeln!(file, "NAME=swarm_name").unwrap();
    writeln!(file, "REPO_URL={}", TEST_REPO_URL).unwrap();
    writeln!(file, "CLONE_PATH={}", clone_path.display()).unwrap();
    writeln!(file, "MOUNT_PATH={}", mount_path.display()).unwrap();
    writeln!(file, "COMPOSE_FILE={}", compose_file_path.display()).unwrap();

    File::create(&compose_file_path).unwrap();

    let cli = CLI {
        tag: "v1.2.3".to_string(),
        name: Some("swarm_name".to_string()),
        socket_path: "/var/run/docker.sock".to_string(),
        repo_url: Some(TEST_REPO_URL.to_string()),
        clone_path: Some(clone_path.display().to_string()),
        mount_path: Some(mount_path.display().to_string()),
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: true,
        swarm_service: Some("swarm_service".to_string()),
    };

    deploy(cli).await;
}

#[tokio::test]
async fn test_swarm_service_cli_and_env() {
    std::env::set_var("SKIP_DEPLOY", "1");
    let temp_dir = tempdir().unwrap();
    let clone_path = temp_dir.path().join("swarm_cli_env_clone");
    let mount_path = temp_dir.path().join("swarm_cli_env_mount");
    let env_path = temp_dir.path().join("test_swarm_cli_env.env");
    let compose_file_path = temp_dir.path().join("swarm-cli-env-compose.yml");
    let mut file = File::create(&env_path).unwrap();
    writeln!(file, "NAME=swarm_cli_env_name").unwrap();
    writeln!(file, "REPO_URL={}", TEST_REPO_URL).unwrap();
    writeln!(file, "CLONE_PATH={}", clone_path.display()).unwrap();
    writeln!(file, "MOUNT_PATH={}", mount_path.display()).unwrap();
    writeln!(file, "COMPOSE_FILE={}", compose_file_path.display()).unwrap();
    writeln!(file, "SWARM_SERVICE=env_service").unwrap();

    File::create(&compose_file_path).unwrap();

    // CLI value should take precedence over env
    let cli = CLI {
        tag: "v1.2.3".to_string(),
        name: Some("swarm_cli_env_name".to_string()),
        socket_path: "/var/run/docker.sock".to_string(),
        repo_url: Some(TEST_REPO_URL.to_string()),
        clone_path: Some(clone_path.display().to_string()),
        mount_path: Some(mount_path.display().to_string()),
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: true,
        swarm_service: Some("cli_service".to_string()),
    };
    deploy(cli).await;

    // Now test with only env value
    let cli_env_only = CLI {
        tag: "v1.2.3".to_string(),
        name: Some("swarm_cli_env_name".to_string()),
        socket_path: "/var/run/docker.sock".to_string(),
        repo_url: Some(TEST_REPO_URL.to_string()),
        clone_path: Some(clone_path.display().to_string()),
        mount_path: Some(mount_path.display().to_string()),
        verbose: 0,
        compose_file: compose_file_path.display().to_string(),
        env_file: env_path.display().to_string(),
        swarm: true,
        swarm_service: None,
    };
    deploy(cli_env_only).await;
}
