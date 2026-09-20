# Configuration

Audience: anyone configuring any service in this repository. Update trigger:
a new or changed environment variable. This file is the single source of
truth — READMEs link here and must not restate these tables.

Backend variables live in `blog/backend/.env` (see `example.env`), frontend
variables in `blog/frontend/.env`.

## Backend — core

| Variable | Purpose | Default |
| --- | --- | --- |
| `DATABASE_URL` | SQLite connection string. | `sqlite:data/blog.db` |
| `JWT_SECRET` | Secret used to sign access and refresh tokens. | — (required) |
| `ACCESS_JWT_EXP_HOURS` | Access token lifetime in hours. | `24` |
| `REFRESH_JWT_EXP_HOURS` | Refresh token lifetime in hours. | `360` |
| `APP_BASE_URL` | Public app URL used by auth email flows. | `http://localhost:5000` |
| `MEDIA_PATH` | Directory for uploaded media. | `./media` |
| `PROJECT_DEMOS_PATH` | Directory for uploaded/extracted project demos and (in `fs` mode) v86 artifacts. | `./project-demos` |

## Backend — demo and v86 limits

| Variable | Purpose | Default |
| --- | --- | --- |
| `PROJECT_DEMO_MAX_ARCHIVE_BYTES` | Max uploaded demo archive size. | built-in limit |
| `PROJECT_DEMO_MAX_EXTRACTED_BYTES` | Max extracted demo size. | built-in limit |
| `PROJECT_DEMO_MAX_FILES` | Max extracted demo file count. | built-in limit |
| `PROJECT_V86_BASE_MAX_BYTES` | Max raw v86 base IMG size. | 2 GiB |
| `PROJECT_V86_GAME_ZIP_MAX_BYTES` | Max compressed v86 game ZIP size. | 500 MiB |
| `PROJECT_V86_GAME_EXTRACTED_MAX_BYTES` | Max expanded game size. | 1 GiB |
| `PROJECT_V86_GAME_MAX_FILES` | Max files in a v86 game ZIP. | 10,000 |
| `PROJECT_V86_UPLOAD_CHUNK_BYTES` | v86 upload chunk size. | 8 MiB |
| `PROJECT_V86_DOWNLOAD_CHUNK_BYTES` | Immutable v86 disk-part size. | 256 KiB |

## Backend — object storage (v86 artifacts)

| Variable | Purpose |
| --- | --- |
| `STORAGE_BACKEND` | Object store for v86 game artifacts: `auto` (R2 when configured, else fs), `r2`, or `fs`. |
| `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET` | Cloudflare R2 credentials, required only when `STORAGE_BACKEND` resolves to `r2`. |
| `R2_PUBLIC_URL` | Public R2 domain used to build absolute v86 artifact URLs in `r2` mode; ignored in `fs` mode. |

Behavior details and the R2↔fs switch procedure:
[../reference/media-and-storage.md](../reference/media-and-storage.md) and
[operations.md](operations.md).

## Backend — CORS and mail

| Variable | Purpose | Default |
| --- | --- | --- |
| `ALLOWED_ORIGIN` or `ALLOWED_ORIGINS` | Comma-separated CORS allow list for restricted browser calls (contact form, media, v86 artifacts). | dev defaults; prod: `https://portfolio.huuthangle.site` |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD` | Optional SMTP transport. | — |
| `SMTP_FROM`, `SMTP_TO` | Sender and contact-form recipient; required when mail transport is enabled. | — |
| `BREVO_API_KEY` | Optional Brevo API transport (takes precedence over SMTP). | — |

Email is optional — the server starts without mail transport when no mail
variables are configured.

## Frontend

| Variable | Purpose | Default |
| --- | --- | --- |
| `API_URL` | Backend URL for server-side SvelteKit requests. Docker: `http://backend:3000`; standalone: `http://localhost:3000`. Never exposed to the browser. | — |
| `BACKEND_ORIGIN` | Public backend origin for direct browser media URLs (e.g. `https://api.huuthangle.site`). Omit to fall back to the `/api/media/...` proxy. | — |
| `ALLOWED_HOSTS` | Comma-separated extra hostnames accepted by the production frontend. Canonical blog host, localhost, and the Fly hostname are always accepted. | — |
| `TRUSTED_ORIGINS` | Comma-separated extra browser origins allowed to make state-changing requests. Blog, portfolio, and local dev origins are included by default. | — |
| `PORT` | Port for the built SvelteKit server. | Docker sets `8080` |
| `BODY_SIZE_LIMIT` | Request body limit for the SvelteKit server. | Docker sets `100M` |
