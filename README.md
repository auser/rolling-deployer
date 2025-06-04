# Deployer for traefik configs

## Project Brief

This project provides a robust deployer for traefik configuration using Docker Compose. It enables rolling upgrades and safe rollbacks of traefik config by versioning configuration directories and updating the docker-compose volume source. The deployer requires a `docker-compose.yml` file for your traefik service, and manages config updates by switching the mounted config directory to the desired version.

- **Rolling upgrades**: Deploy a new config version by specifying a git tag; the deployer clones the config repo at that tag, updates the compose volume, and restarts the service with zero downtime.
- **Rollbacks**: Instantly revert to a previous config version by specifying an older tag; the deployer switches the config mount and restarts the service.
- **Requirements**: You must have a valid `docker-compose.yml` file for your traefik project. The deployer updates the config volume in this file.

## Usage

### With CLI arguments only

```bash
traefik-deployer --tag v1.2.3 --name my-traefik-project \
  --repo-url https://github.com/org/repo.git \
  --mount-path /etc/traefik/dynamic \
  --clone-path /opt/traefik-configs \
  --compose-file ./docker-compose.yml
```

### With .env file integration

You can provide configuration via a `.env` file (default: `.env`). CLI flags always override `.env` values.

Example `.env`:
```bash
REPO_URL=https://github.com/org/repo.git
CLONE_PATH=/opt/traefik-configs
MOUNT_PATH=/etc/traefik/dynamic
COMPOSE_FILE=./docker-compose.yml
NAME=my-traefik-project
SOCKET_PATH=/var/run/docker.sock
```

Then run:
```bash
traefik-deployer --tag v1.2.3
```

Or override any value:
```bash
traefik-deployer --tag v1.2.3 --name another-project
```

## How Rollbacks and Upgrades Work

- **Upgrade**: The deployer clones the config repo at the specified tag into a versioned directory, updates the docker-compose volume to point to this directory, and runs `docker compose up -d --force-recreate` for the traefik service.
- **Rollback**: Specify an older tag to revert; the deployer switches the config mount to the previous version and restarts the service.
- **Cleanup**: Old config directories are automatically cleaned up (keeping the last 3 versions).

## Development

Release a new version:

```bash
git ci -am "Updated version"; cargo release patch --execute --all --no-confirm; git push origin HEAD; git push --tags
```

