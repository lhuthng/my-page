# CI/CD and Infrastructure

Audience: whoever changes the pipeline or VM setup. Update trigger: a
workflow or secret change.

## Pipeline overview

The deploy pipeline lives in `.github/workflows/deploy.yml`.

- Runs on pushes to `master`, only for changes inside `blog/**` or the
  workflow itself.
- `[deploy blog]` in a commit message forces a blog deploy.
- `[manual deploy]` skips the pipeline entirely (for local builds — see
  `scripts/emergency/README.md`).

## Blog deployment

Four jobs: `filter`, `build-push-backend`, `build-push-frontend`,
`deploy-blog`.

Images are built on GitHub Actions and pushed to GHCR:

- `ghcr.io/lhuthng/blog-backend:latest`
- `ghcr.io/lhuthng/blog-frontend:latest`

After both images are available, `deploy-blog` SSHes into the Oracle VM and
runs:

```bash
docker login ghcr.io
docker compose pull
docker compose up -d --remove-orphans
docker image prune -f
```

The workflow then waits briefly, prints backend logs, and verifies the
backend container is still running.

## Required secrets

| Secret | Description |
| --- | --- |
| `VM_HOST` | Oracle VM public IP |
| `VM_USER` | SSH username |
| `VM_SSH_KEY` | Private SSH key for the VM |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` so the VM can pull images |

`GITHUB_TOKEN` is used automatically inside GitHub Actions for pushing images
to GHCR.

## nginx

The nginx sites this repo manages live in `ops/nginx/` (scp'd to
`/etc/nginx/sites-available` by the deploy workflow when changed). The full
topology — which prefixes go to the backend vs the frontend, rate limiting,
TLS/certbot ownership — is documented in
[../docs/architecture/deployment.md](../docs/architecture/deployment.md).

## Hosting history

The blog previously ran on Fly.io before the Cloudflare + Oracle Cloud
migration (April 2026). The migration notes lived outside the repository and
are gone; the current topology is described in
[../docs/architecture/deployment.md](../docs/architecture/deployment.md).
