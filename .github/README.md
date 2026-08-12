# CI/CD and Infrastructure

## Pipeline overview

The deploy pipeline lives in `.github/workflows/deploy.yml`.

It runs on pushes to `master` and only reacts to changes inside `blog/**` or the workflow itself.

You can force a blog deploy by adding `[deploy blog]` to the commit message.

You can skip CI entirely by adding `[manual deploy]` to the commit message —
the whole pipeline is skipped for that push, for when you're building and
deploying locally instead (see `scripts/emergency/README.md`).

## Blog deployment

The workflow has four jobs:

1. `filter`
2. `build-push-backend`
3. `build-push-frontend`
4. `deploy-blog`

The backend and frontend images are built on GitHub Actions and pushed to GHCR:

- `ghcr.io/lhuthng/blog-backend:latest`
- `ghcr.io/lhuthng/blog-frontend:latest`

After both images are available, `deploy-blog` SSHes into the Oracle VM and runs:

```bash
docker login ghcr.io
docker compose pull
docker compose up -d --remove-orphans
docker image prune -f
```

The workflow then waits briefly, prints backend logs, and verifies that the backend container is still running.

## Required secrets

| Secret | Description |
| --- | --- |
| `VM_HOST` | Oracle VM public IP |
| `VM_USER` | SSH username |
| `VM_SSH_KEY` | Private SSH key for the VM |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` so the VM can pull images |

`GITHUB_TOKEN` is used automatically inside GitHub Actions for pushing images to GHCR.

## VM notes

This repo no longer keeps nginx `.conf` files. nginx is managed directly on the server.

Important nginx bits to keep:

```nginx
server {
    server_name blog.huuthangle.site;

    location /media/ {
        proxy_pass http://127.0.0.1:3001/media/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header CF-IPCountry $http_cf_ipcountry;
    }

    location / {
        proxy_pass http://127.0.0.1:5000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header CF-IPCountry $http_cf_ipcountry;
    }
}
```

Optional root redirect:

```nginx
server {
    server_name huuthangle.site;
    return 301 https://blog.huuthangle.site$request_uri;
}
```

If you use rate limiting, the two lines that matter are:

```nginx
limit_req_zone $binary_remote_addr zone=blog:10m rate=5r/s;
limit_req zone=blog burst=20 nodelay;
```

For the current hosting setup and migration context, see:

- `journal-logs/2026-04-09-flyio-cloudflare-migration.md`
- `journal-logs/2026-04-10-oracle-cloud-migration.md`
