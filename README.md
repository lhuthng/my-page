# Blog Platform

Personal blog platform for [Huu Thang](https://github.com/lhuthng), built as a full-stack application with a SvelteKit SSR frontend, a Rust/Axum backend, SQLite persistence, Docker Compose runtime, and GitHub Actions deployment.

This repository also contains an older `portfolio/` project, but this README focuses on the blog system under `blog/`.

## Stack

| Layer | Technology | Purpose |
| --- | --- | --- |
| Frontend | SvelteKit 2, Svelte 5, Bun, Tailwind CSS | Server-rendered blog UI, editor, dashboard, API proxy |
| Backend | Rust 2024, Axum, sqlx, SQLite | REST API, auth, content, media, analytics, admin GraphQL |
| Storage | SQLite and local volumes | Blog database, uploaded media, project demos |
| Runtime | Docker Compose | Backend and frontend containers on one VM |
| Edge | nginx | TLS termination and routing to frontend or media backend |
| CI/CD | GitHub Actions, GHCR, SSH | Build images, push to registry, restart VM containers |

## Repo Layout

```text
my-page/
|-- .github/
|   |-- README.md
|   `-- workflows/deploy.yml
|-- blog/
|   |-- backend/
|   |   |-- migrations/
|   |   |-- src/
|   |   |-- Dockerfile
|   |   |-- Cargo.toml
|   |   `-- readme.md
|   |-- frontend/
|   |   |-- src/
|   |   |-- Dockerfile
|   |   |-- package.json
|   |   `-- README.md
|   |-- docker-compose.yml
|   |-- Makefile
|   `-- README.md
|-- portfolio/
`-- README.md
```

Important paths:

- `blog/frontend`: SvelteKit SSR app, page routes, proxy routes, editor UI, comments, media manager, dashboard.
- `blog/backend`: Axum API, domain/application/infrastructure layers, migrations, media storage, project demo storage.
- `blog/docker-compose.yml`: production-style local runtime with backend and frontend containers.
- `blog/Makefile`: common development, formatting, linting, migration, and Docker commands.
- `.github/workflows/deploy.yml`: current blog deployment pipeline.

## Architecture

Production traffic is intended to flow through Cloudflare DNS to an Oracle Cloud VM running nginx and Docker Compose.

```text
Browser
  -> Cloudflare DNS
     -> blog.huuthangle.site
        -> Oracle Cloud VM
           -> nginx
              -> /media/*        -> backend container via 127.0.0.1:3001
              -> everything else -> frontend container via 127.0.0.1:5000
                                      -> server-side API calls -> http://backend:3000
```

The backend container listens on port `3000` inside Docker and is bound to `127.0.0.1:3001` on the host. The frontend container listens on `8080` inside Docker and is bound to `127.0.0.1:5000` on the host.

The frontend uses three request paths:

1. SvelteKit server-side loads call `API_URL`, normally `http://backend:3000` in Docker.
2. Browser API calls go to `/api/...`, then SvelteKit proxies them to the backend.
3. Media URLs use `BACKEND_ORIGIN` when configured, so the browser can fetch `/media/*` directly through nginx without passing large files through SvelteKit.

Persistent state lives in mounted directories:

- `blog/backend/data`: SQLite database.
- `blog/backend/media`: uploaded images, videos, audio, models, Lottie files, covers, and avatars.
- `blog/backend/project-demos`: uploaded or extracted project demo assets.

## Backend Design

The backend is organized around a small clean-architecture style split:

```text
blog/backend/src/
|-- domain/          # Entities, value objects, and domain errors
|-- application/     # Commands and service-level use cases
`-- infrastructure/  # Axum routes, SQLite persistence, auth, mail, media, GraphQL
```

REST is the primary API surface. GraphQL is available at `/graphql` for protected moderator/admin workflows. SQLite migrations run automatically on backend startup, and can also be run manually with `make migrate`.

## Features

### Reader Experience

- Featured posts and projects on the homepage.
- Public archives for posts, projects, tags, series, and author profiles.
- Searchable and paginated discovery pages for larger content sets.
- Rich post pages with author metadata, tags, publish/update dates, related posts, and series navigation.
- Responsive table of contents generated from article headings.
- Like/upvote tracking, delayed view tracking, and share actions for copy link, X/Twitter, and LinkedIn.
- SEO-friendly pages with canonical links, descriptions, Open Graph metadata, Twitter cards, and cover image or cover video metadata.

### Writing And Editorial Tools

- Draft-first editing flow for posts and projects.
- Separate draft and public views while editing existing content.
- Live rendered preview inside the editor shell.
- Slug availability checks before create/update actions.
- Cover uploads for images and videos, including Open Graph image timing for video covers.
- Inline media detection for content that references uploaded assets.
- Automatic bundling of new inline media files into post and project create/update requests.
- Post metadata management for title, slug, excerpt, tags, series membership, and related posts.
- Project metadata management for links, demo dimensions, demo URL, demo ZIP, and demo type.
- Project demos support HTML5, WebGL, embeds, downloads, videos, or no demo.
- Draft creation prompt with options to publish immediately or continue editing in the dashboard.

### Content Rendering

- Markdown rendering with custom extensions for richer technical writing.
- Numbered headings and generated article navigation.
- Syntax highlighting for code-heavy posts.
- KaTeX support for math notation.
- Uploaded media shortcuts for images, audio, video, models, and Lottie assets.
- Custom blocks for YouTube, iframes, embedded apps, reveal sections, and named containers.
- Expandable images and styled media containers.
- Mention and kaomoji rendering in supported content.

### Comments

- Threaded comments on post pages.
- Guest identities for visitors who are not logged in.
- Logged-in commenting with user avatar support.
- Reply flows with expandable reply threads and load-more behavior.
- Comment composer with live preview.
- Toolbar actions for headings, bold, italics, and inline code.
- Mention autocomplete.
- GIF and kaomoji search drawers.
- Focused frontend tests for comment syntax and thread behavior.

### Media

- Dashboard media manager for uploads, browsing, search, and edits.
- Upload previews before committing files.
- Detail panel for inspecting and editing selected media.
- Alias management for stable media references.
- Revert and delete actions for managed assets.
- Shareable media links and direct file serving.
- Dedicated validation rules for general media, avatars, and covers.
- Supported formats include PNG, GIF, WebP, JPEG, MP4, WebM, MP3, OGG, GLB, and Lottie.

### Auth, Admin, And Operations

- Registration, login, logout, token refresh, email verification, resend verification, forgot password, and reset password.
- Role-based access for user, moderator, and admin workflows.
- Dashboard overview with content counts, user counts, comment counts, role breakdown, growth charts, top posts, and visitor countries.
- Post and project management dashboards with search, pagination, draft/published status, edit actions, and create actions.
- Highlight management for choosing homepage featured posts and projects.
- User dashboard with search, role filters, role counts, and admin role management.
- Database dashboard for inspecting users, posts, projects, media, comments, tags, series, sessions, notifications, and analytics data.
- Tag editing and deletion from the admin database tools.
- Backup download for the SQLite database, uploaded media, and project demos.
- Optional contact form delivery through SMTP or Brevo.

## Requirements

For full-stack local development:

- Docker and Docker Compose.
- Bun for frontend development.
- Rust and Cargo for backend development.
- SQLite-compatible `DATABASE_URL`.
- `sqlx-cli` if you want to run migrations manually.

Optional integrations:

- SMTP credentials or `BREVO_API_KEY` for email.
- rclone config for remote backups.
- nginx, DNS, TLS, and firewall setup for production hosting.

## Running The Blog

### Full Stack With Docker Compose

From the repository root:

```bash
cd blog
make setup
```

`make setup` creates `blog/backend/data` and copies example environment files if they do not already exist:

- `blog/backend/example.env` -> `blog/backend/.env`
- `blog/frontend/example.env` -> `blog/frontend/.env`

Edit both `.env` files before starting the stack. Then run:

```bash
docker compose up -d --build
```

Or use the Makefile wrapper:

```bash
make docker-up
```

The frontend is available at:

```text
http://localhost:5000
```

The backend is bound to localhost only:

```text
http://127.0.0.1:3001
```

Inside Docker, the frontend reaches the backend at:

```text
http://backend:3000
```

### Standalone Development

Run the backend directly:

```bash
cd blog
make backend
```

This runs `cargo run --bin backend` from `blog/backend`. The backend reads `blog/backend/.env` and runs pending migrations on startup.

Run the frontend directly:

```bash
cd blog
make frontend
```

This runs `bun run dev` from `blog/frontend`. For standalone frontend development, set `API_URL` in `blog/frontend/.env` to the backend URL, usually:

```text
API_URL=http://localhost:3000
```

## Makefile Commands

Run these from `blog/`.

| Command | What it does |
| --- | --- |
| `make setup` | Creates the data directory and copies missing example env files. |
| `make backend` | Runs the Rust backend locally with Cargo. |
| `make frontend` | Runs the SvelteKit dev server with Bun. |
| `make migrate` | Runs sqlx migrations manually. |
| `make docker-up` | Starts Docker Compose services in the background. |
| `make docker-down` | Stops Docker Compose services. |
| `make docker-build` | Rebuilds Docker images locally. |
| `make lint` | Runs backend clippy with warnings denied and frontend Prettier check. |
| `make fmt` | Runs Cargo fmt and frontend Prettier write. |

Manual migration example:

```bash
cd blog
make migrate
```

By default this uses:

```text
sqlite://data/blog.db
```

Override it with `DB_URL` when needed:

```bash
make migrate DB_URL=sqlite://path/to/blog.db
```

## Configuration

### Backend

Common backend variables live in `blog/backend/.env`.

| Variable | Purpose |
| --- | --- |
| `DATABASE_URL` | SQLite connection string, for example `sqlite:data/blog.db`. |
| `JWT_SECRET` | Secret used to sign access and refresh tokens. |
| `ACCESS_JWT_EXP_HOURS` | Access token lifetime in hours. |
| `REFRESH_JWT_EXP_HOURS` | Refresh token lifetime in hours. |
| `MEDIA_PATH` | Directory for uploaded media. |
| `PROJECT_DEMOS_PATH` | Directory for uploaded and extracted project demos. |
| `PROJECT_DEMO_MAX_ARCHIVE_BYTES` | Optional max uploaded demo archive size. |
| `PROJECT_DEMO_MAX_EXTRACTED_BYTES` | Optional max extracted demo size. |
| `PROJECT_DEMO_MAX_FILES` | Optional max extracted demo file count. |
| `PROJECT_V86_BASE_MAX_BYTES` | Optional maximum raw v86 base IMG size; defaults to 2 GiB. |
| `PROJECT_V86_GAME_ZIP_MAX_BYTES` | Optional maximum compressed v86 game ZIP size; defaults to 500 MiB. |
| `PROJECT_V86_GAME_EXTRACTED_MAX_BYTES` | Optional maximum expanded game size; defaults to 1 GiB. |
| `PROJECT_V86_GAME_MAX_FILES` | Optional maximum number of files in a v86 game ZIP; defaults to 10,000. |
| `PROJECT_V86_UPLOAD_CHUNK_BYTES` | v86 upload chunk size; defaults to 8 MiB. |
| `PROJECT_V86_DOWNLOAD_CHUNK_BYTES` | Immutable v86 disk-part size; defaults to 256 KiB. |
| `PROJECT_V86_XORRISO_BIN` | Path to `xorriso`, used to build immutable game ISOs. |
| `APP_BASE_URL` | Public app URL used by auth email flows. |
| `ALLOWED_ORIGIN` or `ALLOWED_ORIGINS` | CORS allow list for restricted browser calls. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD` | Optional SMTP email transport. |
| `SMTP_FROM`, `SMTP_TO` | Required when email transport is enabled. |
| `BREVO_API_KEY` | Optional Brevo API email transport. |

Email is optional. The server starts without mail transport when no mail variables are configured.

### Frontend

Common frontend variables live in `blog/frontend/.env`.

| Variable | Purpose |
| --- | --- |
| `API_URL` | Backend URL for server-side SvelteKit requests. |
| `BACKEND_ORIGIN` | Public origin used to build direct browser media URLs. |
| `PORT` | Port for the built SvelteKit server. Docker sets this to `8080`. |
| `BODY_SIZE_LIMIT` | Request body limit for the SvelteKit server. Docker sets this to `100M`. |

In Docker, `API_URL` should be:

```text
http://backend:3000
```

In standalone local development, it is usually:

```text
http://localhost:3000
```

## Deployment

The current deployment path is designed for one Oracle Cloud VM.

Pushes to `master` trigger [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml) when files under `blog/**` or `.github/workflows/**` change. The workflow can also be run manually with `workflow_dispatch`.

The workflow:

1. Builds the backend Docker image from `blog/backend`.
2. Builds the frontend Docker image from `blog/frontend`.
3. Pushes both images to GitHub Container Registry.
4. SSHes into the VM.
5. Logs the VM into GHCR with `GHCR_TOKEN`.
6. Runs `docker compose pull` from `~/MyPage/blog`.
7. Runs `docker compose up -d --remove-orphans`.
8. Prunes unused images.
9. Shows backend and frontend logs.
10. Checks that both containers are running.

Published image tags:

```text
ghcr.io/lhuthng/blog-backend:latest
ghcr.io/lhuthng/blog-frontend:latest
```

Required GitHub secrets:

| Secret | Purpose |
| --- | --- |
| `VM_HOST` | Oracle VM public IP or hostname. |
| `VM_USER` | SSH username. |
| `VM_SSH_KEY` | Private key used for SSH deployment. |
| `GHCR_TOKEN` | Token that lets the VM pull packages from GHCR. |

## Production Expectations

The repo assumes several host-level pieces already exist:

- The VM has Docker and Docker Compose installed.
- The repo is checked out on the VM at `~/MyPage/blog`, matching the deploy workflow.
- nginx is installed and configured outside this repo.
- nginx terminates TLS and proxies normal traffic to `127.0.0.1:5000`.
- nginx proxies `/media/*` to `127.0.0.1:3001`.
- DNS points `blog.huuthangle.site` at the VM.
- The firewall only exposes the intended public ports.
- `blog/backend/.env` and `blog/frontend/.env` exist on the VM with production values.
- Persistent directories for data, media, and demos are preserved across deploys.

Migrations run automatically when the backend starts, so a failed migration can prevent the backend container from becoming healthy. The deploy workflow prints backend logs specifically to make migration failures visible.

## Limitations

- Deployment is VM-specific and expects the remote path `~/MyPage/blog`.
- Docker images use the mutable `latest` tag instead of immutable release tags.
- SQLite and local media volumes make this a single-node application.
- Horizontal scaling would require changes to database, session, media, and demo storage.
- nginx config, TLS certificates, VM provisioning, and firewall rules are not managed by this repository.
- Uploaded media and project demos are local state, so backups are required before VM rebuilds or migrations.
- Admin and moderator access depends on role data in the database. Initial admin setup is not fully automated by this README.
- Email verification, password reset, and contact form delivery require SMTP or Brevo configuration.
- The portfolio project is not part of this blog runtime or deployment path.

## Deeper Docs

- [blog/README.md](blog/README.md)
- [blog/backend/readme.md](blog/backend/readme.md)
- [blog/frontend/README.md](blog/frontend/README.md)
- [.github/README.md](.github/README.md)
