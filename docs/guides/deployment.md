# Deployment

Audience: whoever ships the blog. Update trigger: a pipeline or deploy-path
change.

## CI/CD (default path)

Pushes to `master` trigger `.github/workflows/deploy.yml` when files under
`blog/**` or the workflow change. The workflow builds both Docker images,
pushes them to GHCR, then SSHes into the VM and runs `docker compose pull &&
up -d --remove-orphans`, prunes, prints backend logs, and verifies both
containers are running.

- Images: `ghcr.io/lhuthng/blog-backend:latest`, `ghcr.io/lhuthng/blog-frontend:latest`
- Required GitHub secrets: `VM_HOST`, `VM_USER`, `VM_SSH_KEY`, `GHCR_TOKEN`
- `[deploy blog]` in a commit message forces a deploy; `[manual deploy]`
  skips the pipeline entirely.

Pipeline details and CI/CD reference: [../../.github/README.md](../../.github/README.md).
Topology: [../architecture/deployment.md](../architecture/deployment.md).

## Manual emergency path

`scripts/emergency/deploy.sh` builds and pushes images from the local machine
(docker buildx, `linux/amd64`) when CI is unavailable, or for any push tagged
`[manual deploy]`. Frontend builds are cheap; the Rust backend cross-compiles
via QEMU and is slow — use it only when Rust changes. One-time setup and
usage: [../../scripts/emergency/README.md](../../scripts/emergency/README.md).

## Production expectations

- Docker + Docker Compose installed on the VM; repo at `~/MyPage/blog`.
- nginx installed and configured (see `ops/nginx/` and
  [../architecture/deployment.md](../architecture/deployment.md)).
- `blog/backend/.env` and `blog/frontend/.env` exist on the VM with
  production values (variables: [configuration.md](configuration.md)).
- Persistent directories (`data`, `media`, `project-demos`) survive deploys.

Migrations run automatically on startup, so a failed migration keeps the
backend container unhealthy; the deploy workflow prints backend logs to make
that visible.
