# Architecture Overview

Audience: anyone changing the blog runtime. Update trigger: a change to the
request flow, the deployment topology, or the service split.

## System context

The blog is a full-stack application: a SvelteKit SSR frontend and a
Rust/Axum backend, running as two Docker containers on one Oracle Cloud VM
behind nginx, with SQLite and local volumes for state.

| Layer | Technology | Purpose |
| --- | --- | --- |
| Frontend | SvelteKit 2, Svelte 5, Bun, Tailwind CSS | Server-rendered blog UI, editor, dashboard, API proxy |
| Backend | Rust 2024, Axum, sqlx, SQLite | REST API, auth, content, media, analytics, admin GraphQL |
| Storage | SQLite and local volumes | Database, uploaded media, project demos, v86 artifacts |
| Runtime | Docker Compose | Backend and frontend containers on one VM |
| Edge | nginx | TLS termination and routing to frontend or media backend |
| CI/CD | GitHub Actions, GHCR, SSH | Build images, push to registry, restart VM containers |

The repository also contains `portfolio/` — a static React site deployed to
Cloudflare Pages, independent of the blog runtime.

## Request flow

Production traffic flows through Cloudflare DNS to the VM:

```text
Browser
  -> Cloudflare DNS
     -> huuthangle.site            (blog.huuthangle.site 301s here)
        -> Oracle Cloud VM
           -> nginx
              -> /media/*        -> backend container via 127.0.0.1:3001
              -> everything else -> frontend container via 127.0.0.1:5000
                                      -> server-side API calls -> http://backend:3000
```

The backend container listens on `3000` inside Docker and is bound to
`127.0.0.1:3001` on the host. The frontend container listens on `8080`
inside Docker and is bound to `127.0.0.1:5000`.

The frontend uses three request paths:

1. SvelteKit server-side loads call `API_URL`, normally `http://backend:3000`
   in Docker.
2. Browser API calls go to `/api/...`, then SvelteKit proxies them to the
   backend.
3. Media URLs use `BACKEND_ORIGIN` when configured, so the browser fetches
   `/media/*` directly through nginx instead of pulling large files through
   SvelteKit.

## State

Persistent state lives in mounted directories that must survive deploys:

- `blog/backend/data` — SQLite database.
- `blog/backend/media` — uploaded images, videos, audio, models, Lottie
  files, covers, avatars.
- `blog/backend/project-demos` — uploaded/extracted project demo assets and
  (in `fs` storage mode) v86 game artifacts.

## Where to go deeper

- Backend layer model and module map: [backend.md](backend.md)
- Frontend structure: [frontend.md](frontend.md)
- Database schema and migration policy: [data-model.md](data-model.md)
- nginx/TLS/DNS/VM topology: [deployment.md](deployment.md)
- Environment variables: [../guides/configuration.md](../guides/configuration.md)
- REST surface: [../reference/api-rest.md](../reference/api-rest.md)
