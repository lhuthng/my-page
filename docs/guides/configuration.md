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
| `PORT` | Port the backend binds. Unusable values (empty, non-numeric, or `0`, which the OS reads as *any free port*) fall back to the default rather than failing the bind. | `5174` |

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

## Backend — media storage (uploaded files)

Everything uploaded is written to `MEDIA_PATH`. The one exception is audiobook
audio, which when `AUDIO_BACKEND=r2` is *also* stored in the shared bucket under
the `audio/` prefix. Reads try the bucket first and fall back to disk, so a
half-migrated tree keeps working. Keys are store-independent and
content-addressed: the disk path is `MEDIA_PATH.join(key)` and the object is that
same key with the prefix, so the backfill is a plain copy and nothing has to move
on disk.

"Audiobook audio" is exactly the media an `audiobook_tracks` row points at — not
every row whose content type is audio, which would also take in a media-library
sound file and the media of a track that has been replaced. Images, covers,
avatars, video and models are **disk-only**: they have no object key at all, so
nothing in the read, write, sync or backfill paths can reach the bucket for them.

| Variable | Purpose | Default |
| --- | --- | --- |
| `AUDIO_BACKEND` | `fs` serves audio from disk only; `r2` also stores audio in the bucket and serves from it. There is no `auto` — see below. | `fs` |
| `AUDIO_READ` | `redirect` (302 to the object's public URL) or `proxy` (stream through the backend). | `redirect` |

Connection details are the `R2_*` variables in the section below: the same bucket
as the v86 artifacts, told apart by the prefix. `AUDIO_BACKEND=r2` requires
`R2_PUBLIC_URL`, because that is where audio is served from.

There is deliberately no `auto` here, unlike `STORAGE_BACKEND`: `auto` may pick R2
as a side effect of the credentials existing, and audio moving to the bucket
should be a decision rather than something that happens because R2 was configured
for the artifacts.

Audio is public-by-URL at `R2_PUBLIC_URL/audio/...`. Nothing is signed. The
migration procedure is in [operations.md](operations.md); behavior details in
[../reference/media-and-storage.md](../reference/media-and-storage.md).

## Backend — object storage (v86 artifacts and audio)

One bucket holds both. The v86 artifacts live under `v86/`; audiobook audio lives
under `audio/`. Nothing else is in the bucket. They are switched independently —
`STORAGE_BACKEND` here, `AUDIO_BACKEND` above — so either can be on the bucket or
on the disk on its own.

| Variable | Purpose |
| --- | --- |
| `STORAGE_BACKEND` | Object store for v86 game artifacts: `auto` (R2 when configured, else fs), `r2`, or `fs`. |
| `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET` | Cloudflare R2 credentials, required when `STORAGE_BACKEND` resolves to `r2` or when `AUDIO_BACKEND=r2`. |
| `R2_ENDPOINT` | Optional endpoint override, e.g. a local S3-compatible server for dev. Defaults to `https://{R2_ACCOUNT_ID}.r2.cloudflarestorage.com`. |
| `R2_PUBLIC_URL` | Public R2 domain. Builds absolute v86 artifact URLs in `r2` mode, and is where audio is served from whenever `AUDIO_BACKEND=r2` — so both require it, and it is ignored in `fs` mode. |

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
| `API_URL` | Backend URL for server-side SvelteKit requests. Docker: `http://backend:3000`; standalone: `http://localhost:5174`. Never exposed to the browser. | — |
| `BACKEND_ORIGIN` | Public backend origin for direct browser media URLs (e.g. `https://api.huuthangle.site`). Omit to fall back to the `/api/media/...` proxy. | — |
| `ALLOWED_HOSTS` | Comma-separated extra hostnames accepted by the production frontend. Canonical blog host, localhost, and the Fly hostname are always accepted. | — |
| `TRUSTED_ORIGINS` | Comma-separated extra browser origins allowed to make state-changing requests. Blog, portfolio, and local dev origins are included by default. | — |
| `PORT` | Port for the dev server (`vite.config.js`) and for the built SvelteKit server. | `5175` dev; Docker sets `8080` |
| `BODY_SIZE_LIMIT` | Request body limit for the SvelteKit server. | Docker sets `100M` |

Local development pairs the backend on `5174` with the frontend dev server on
`5175`, which is the pair both `example.env` files are written for; `make dev`
starts the two together. `PORT` moves either side, and Docker pins its own ports
(`3000` for the backend container, `8080` for the frontend).
