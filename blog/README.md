# Blog

A full-stack personal blog — SvelteKit frontend and Rust/Axum backend, both running as Docker containers on the same Oracle Cloud VM, sitting behind nginx.

## Architecture

```
Browser
  ├── /media/*        → nginx → localhost:3001 → Rust backend   (bypasses SvelteKit)
  └── everything else → nginx → localhost:5000 → SvelteKit
                                                      └── server-side calls → http://backend:3000 (Docker internal)
```

nginx handles TLS termination and routes traffic to two upstream targets: port `5000` for the SvelteKit frontend and port `3001` exclusively for media files served by the Rust backend. Server-side data fetching inside SvelteKit goes over the Docker bridge network directly to `http://backend:3000` — never through nginx.

## Docker Services

Defined in `docker-compose.yml`:

| Service | Image | Internal port | Bound to host |
|---|---|---|---|
| `backend` | Rust/Axum | 3000 | `127.0.0.1:3001` |
| `frontend` | SvelteKit/Bun | 8080 | `127.0.0.1:5000` |

**backend** mounts `./backend/data:/app/data` (SQLite database) and `./backend/media:/app/media` (uploaded files).

**frontend** depends on `backend` and shares the same `appnet` bridge network, which is how `http://backend:3000` resolves.

## Local Development

```bash
cp blog/backend/example.env blog/backend/.env
cp blog/frontend/example.env blog/frontend/.env
# edit both .env files
cd blog
docker compose up -d --build
```

Frontend is available at `http://localhost:5000`.

## Environment Variables

### Backend (`backend/.env`)

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | SQLite connection string | `sqlite:data/blog.db` |
| `JWT_SECRET` | Secret used to sign JWTs | *(required)* |
| `ACCESS_JWT_EXP_HOURS` | Access token lifetime (hours) | `24` |
| `REFRESH_JWT_EXP_HOURS` | Refresh token lifetime (hours) | `360` |
| `MEDIA_PATH` | Directory for uploaded files | `./media` |
| `ALLOWED_ORIGIN` | CORS allowed origin for `/mail/contact-form` | `https://portfolio.huuthangle.site` |
| `SMTP_HOST` | SMTP server hostname | *(optional)* |
| `SMTP_PORT` | SMTP server port | *(optional)* |
| `SMTP_USERNAME` | SMTP login username | *(optional)* |
| `SMTP_PASSWORD` | SMTP login password | *(optional)* |
| `SMTP_FROM` | Sender address | *(optional)* |
| `SMTP_TO` | Recipient address | *(optional)* |

### Frontend (`frontend/.env`)

| Variable | Description |
|---|---|
| `API_URL` | Backend URL for server-side calls — Docker internal: `http://backend:3000` |
| `BACKEND_ORIGIN` | Public backend origin for browser media URLs (e.g. `https://blog.huuthangle.site`) |
| `PORT` | Port the SvelteKit server listens on (`8080`) |

## Sub-projects

- [`backend/README.md`](backend/README.md)
- [`frontend/README.md`](frontend/README.md)