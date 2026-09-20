# Setup

Audience: anyone running the blog locally. Update trigger: a new prerequisite
or a change to the bootstrap steps.

## Prerequisites

For full-stack local development:

- Docker and Docker Compose.
- Bun for frontend development.
- Rust and Cargo for backend development.
- SQLite-compatible `DATABASE_URL`.
- `sqlx-cli` if you want to run migrations manually.

Optional integrations:

- SMTP credentials or `BREVO_API_KEY` for email.
- rclone config for remote backups.

## Full stack with Docker Compose

From the repository root:

```bash
cd blog
make setup
```

`make setup` creates `blog/backend/data` and copies the example environment
files if they do not already exist:

- `blog/backend/example.env` → `blog/backend/.env`
- `blog/frontend/example.env` → `blog/frontend/.env`

Edit both `.env` files, then:

```bash
docker compose up -d --build   # or: make docker-up
```

Endpoints:

- Frontend: `http://localhost:5000`
- Backend (localhost only): `http://127.0.0.1:3001`
- Backend from inside Docker: `http://backend:3000`

## Standalone development

Backend:

```bash
cd blog
make backend        # cargo run --bin backend from blog/backend
```

The backend reads `blog/backend/.env` and runs pending migrations on startup.

Frontend:

```bash
cd blog
make frontend       # bun run dev from blog/frontend
```

For standalone frontend development set `API_URL=http://localhost:3000` in
`blog/frontend/.env` (the backend must be running separately).

Every environment variable is listed in
[configuration.md](configuration.md); daily commands are in
[development.md](development.md).
