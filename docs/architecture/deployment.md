# Deployment Topology

Audience: whoever operates the VM or changes the pipeline. Update trigger: a
change to nginx, DNS, TLS, or the container topology.

## Topology

- One Oracle Cloud VM runs Docker Compose with two services:
  `backend` (Rust/Axum, internal 3000, bound to `127.0.0.1:3001`) and
  `frontend` (SvelteKit/Bun, internal 8080, bound to `127.0.0.1:5000`).
- The repo is checked out at `~/MyPage/blog` on the VM, matching the deploy
  workflow's `docker compose` working directory.
- nginx terminates TLS and proxies:
  - `/media/*` → `127.0.0.1:3001` (backend, direct browser media)
  - `/v86/`, `/games/s/`, `/project-demos/` → `127.0.0.1:3001` (needed for
    `fs` storage mode)
  - everything else → `127.0.0.1:5000` (frontend)
- nginx sites managed from this repo live in `ops/nginx/` and are scp'd to
  `/etc/nginx/sites-available` by the deploy workflow when they change.
  Certificates are NOT managed here — certbot on the VM owns
  `/etc/letsencrypt`; a new subdomain needs a one-time
  `sudo certbot certonly --nginx -d <subdomain>`.
- DNS points `huuthangle.site` at the VM; `blog.huuthangle.site` 301s to the
  apex.

## Rate limiting

If used, the two lines that matter:

```nginx
limit_req_zone $binary_remote_addr zone=blog:10m rate=5r/s;
limit_req zone=blog burst=20 nodelay;
```

## Pipeline

Pushes to `master` trigger `.github/workflows/deploy.yml` for changes under
`blog/**` or the workflow itself. It builds both images, pushes them to GHCR
(`ghcr.io/lhuthng/blog-backend:latest`, `ghcr.io/lhuthng/blog-frontend:latest`),
then SSHes to the VM: `docker compose pull && up -d --remove-orphans`, prune,
print backend logs, verify containers.

Details and the manual emergency path: [../../.github/README.md](../../.github/README.md)
and [../guides/deployment.md](../guides/deployment.md).

## Limits

- Single node: SQLite + local media volumes.
- Images use the mutable `latest` tag.
- nginx, TLS certificates, VM provisioning, and firewall rules outside
  `ops/nginx/` are not managed by this repository.
