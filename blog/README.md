# Blog

A full-stack personal blog — SvelteKit frontend and Rust/Axum backend, both
running as Docker containers on the same Oracle Cloud VM, sitting behind
nginx.

## Architecture

```text
Browser
  ├── /media/*        → nginx → localhost:3001 → Rust backend   (bypasses SvelteKit)
  └── everything else → nginx → localhost:5000 → SvelteKit
                                                      └── server-side calls → http://backend:3000 (Docker internal)
```

The full request-flow explanation, nginx topology, and rate limiting:
[../docs/architecture/overview.md](../docs/architecture/overview.md) and
[../docs/architecture/deployment.md](../docs/architecture/deployment.md).

## Docker services

Defined in `docker-compose.yml`:

| Service | Image | Internal port | Bound to host |
|---|---|---|---|
| `backend` | Rust/Axum | 3000 | `127.0.0.1:3001` |
| `frontend` | SvelteKit/Bun | 8080 | `127.0.0.1:5000` |

`backend` mounts `./backend/data` (SQLite) and `./backend/media` (uploads);
`frontend` shares the `appnet` bridge network with it, which is how
`http://backend:3000` resolves.

## Run it

```bash
cp backend/example.env backend/.env
cp frontend/example.env frontend/.env
# edit both
docker compose up -d --build
```

Frontend at `http://localhost:5000`. Makefile targets and standalone
development: [../docs/guides/setup.md](../docs/guides/setup.md) and
[../docs/guides/development.md](../docs/guides/development.md).

## Configuration

Environment variables (backend and frontend) are documented once in
[../docs/guides/configuration.md](../docs/guides/configuration.md).

## Sub-projects

- [backend/](backend/README.md)
- [frontend/](frontend/README.md)
