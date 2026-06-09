# my-page

Personal website monorepo for [Huu Thang](https://github.com/lhuthng).

## Overview

This repo currently contains two projects:

| Project | Domain | Stack | Hosting |
| --- | --- | --- | --- |
| Blog | [blog.huuthangle.site](https://blog.huuthangle.site) | SvelteKit SSR + Rust/Axum + SQLite | Oracle Cloud VM with Docker Compose |
| Portfolio | [portfolio.huuthangle.site](https://portfolio.huuthangle.site) | React 19 + Tailwind + GSAP + Lottie | Cloudflare Pages |

The WebSocket relay was moved to its own repo:
[lhuthng/ws-relay-go](https://github.com/lhuthng/ws-relay-go)

## Repo layout

```text
my-page/
├── .github/
│   ├── README.md
│   └── workflows/deploy.yml
├── blog/
│   ├── backend/
│   ├── frontend/
│   └── docker-compose.yml
├── journal-logs/
├── portfolio/
└── README.md
```

## Blog architecture

```text
browser
  -> Cloudflare DNS
    -> blog.huuthangle.site
      -> Oracle Cloud VM
        -> nginx
          -> SvelteKit frontend (:5000)
            -> Rust backend over Docker network (:3000)
          -> media proxy to backend (:3001)
```

The backend is not exposed directly to the public internet. nginx terminates TLS and proxies requests to the frontend container. The frontend talks to the backend over the Docker network.

## Deployment

`master` pushes trigger [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml). The workflow only handles the blog:

1. Build backend and frontend images on GitHub Actions.
2. Push both images to GHCR.
3. SSH into the Oracle VM.
4. Run `docker compose pull` and `docker compose up -d`.

Cloudflare Pages deploys `portfolio/` automatically from GitHub.

Required GitHub secrets for the blog deploy:

| Secret | Purpose |
| --- | --- |
| `VM_HOST` | Oracle VM public IP |
| `VM_USER` | SSH username |
| `VM_SSH_KEY` | Private key for SSH access |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` so the VM can pull images |

More deployment notes live in [`.github/README.md`](.github/README.md).

## Local development

### Blog

```bash
cd blog
make setup
docker compose up -d --build
```

Useful commands:

```bash
cd blog
make docker-up
make docker-down
make backend
make frontend
make migrate
```

### Portfolio

```bash
cd portfolio
bun install
bun dev
```

## Docs

- [blog/README.md](blog/README.md)
- [portfolio/README.md](portfolio/README.md)
- [.github/README.md](.github/README.md)
- [journal-logs/](journal-logs/)
