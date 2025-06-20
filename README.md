# Rolling Deployer for Docker Compose Services

## Project Brief

This project provides a robust deployer for Docker Compose services that use volume mounts. It enables rolling upgrades and safe rollbacks of containerized applications by versioning configuration directories and updating the docker-compose volume source. The deployer requires a `docker-compose.yml` file for your service, and manages config updates by switching the mounted config directory to the desired version.

- **Rolling upgrades**: Deploy a new config version by specifying a git tag; the deployer clones the config repo at that tag, updates the compose volume, and restarts the service with zero downtime.
- **Rollbacks**: Instantly revert to a previous config version by specifying an older tag; the deployer switches the config mount and restarts the service.
- **Requirements**: You must have a valid `docker-compose.yml` file for your project. The deployer updates the config volume in this file.

## Usage

### With CLI arguments only

```bash
rolling-deployer --tag v1.2.3 --name my-project \
  --repo-url https://github.com/org/repo.git \
  --mount-path /etc/myapp/config \
  --clone-path /opt/configs \
  --compose-file ./docker-compose.yml
```

### With .env file integration

You can provide configuration via a `.env` file (default: `.env`). CLI flags always override `.env` values.

Example `.env`:

```bash
REPO_URL=https://github.com/org/repo.git
CLONE_PATH=/opt/configs
MOUNT_PATH=/etc/myapp/config
COMPOSE_FILE=./docker-compose.yml
NAME=my-project
SOCKET_PATH=/var/run/docker.sock
```

After a successfully deploy, the `CLONE_PATH` directory will be populated with the `REPO_URL` and `TAG` directory.

```bash
/opt/configs/v0.1.0/
/opt/configs/v0.1.1/
# Then we'll have a soft-link to `current`:
/opt/configs/current -> /opt/configs/v0.1.1
```

Then run:

```bash
rolling-deployer --tag v0.1.1
```

Or override any value:

```bash
rolling-deployer --tag v0.1.1 --name another-project
```

See how it works below, however the cursory version is that if we need to rollback, we can just run `rolling-deployer --tag v0.1.0` with the previous tag and it will switch the config mount to the previous version and restart the service.

## How Rollbacks and Upgrades Work

- **Upgrade**: The deployer clones the config repo at the specified tag into a versioned directory, updates the docker-compose volume to point to this directory, and runs `docker compose up -d --force-recreate` for the service.
- **Rollback**: Specify an older tag to revert; the deployer switches the config mount to the previous version and restarts the service.
- **Cleanup**: Old config directories are automatically cleaned up (keeping the last 3 versions).


## Development

Release a new version:

```bash
git ci -am "Updated version"; cargo release patch --execute --all --no-confirm; git push origin HEAD; git push --tags
```

