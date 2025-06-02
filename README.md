# Deployer for traefik configs

## Usage

```bash
traefik-deployer --help
```

```bash
traefik-deployer --tag v1.2.3 --name my-traefik-project --repo-url https://github.com/org/repo.git --mount-path /opt/configs
```

```bash
traefik-deployer --tag v1.2.3 --name my-traefik-project
```

## Configuration

The configuration is done through a `.env` file. If no CLI args are provided, the configuration will be loaded from the `.env` file.

The `.env` file should contain the following variables:

```bash
REPO_URL=https://github.com/org/repo.git
MOUNT_PATH=/opt/configs
```

## Development

Release a new version:

```bash
git ci -am "Updated version"; cargo release patch --execute --all --no-confirm; git push origin HEAD; git push --tags
```

