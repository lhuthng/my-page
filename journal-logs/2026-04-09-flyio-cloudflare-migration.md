# Migration Journal: VPS → Fly.io + Cloudflare
**Date:** 2026-04-09  
**Author:** Huu Thang

---

## Context

The entire website was previously running on a single VPS with Docker + nginx + Certbot. The goal was to migrate to free/cheap cloud infrastructure while keeping the same domain names and functionality.

### Services

| Service | Tech | Old hosting |
|---|---|---|
| `portfolio` | React (Bun), static output to `dist/` | VPS + nginx |
| `blog/frontend` | SvelteKit SSR, `svelte-adapter-bun`, Docker | VPS + Docker + nginx |
| `blog/backend` | Rust/Axum + SQLite + local media folder, Docker | VPS + Docker |
| `socket-server` | Node.js WebSocket proxy (no Dockerfile existed) | VPS + nginx |

---

## Target Architecture

| Service | New hosting | Why |
|---|---|---|
| `portfolio` | Cloudflare Pages | Truly free, unlimited CDN, auto-deploy from GitHub |
| `blog/frontend` | Fly.io (shared-cpu-1x, 256MB) | Docker-native, always-on free machine |
| `blog/backend` | Fly.io (shared-cpu-1x, 256MB) + 3GB volume | Persistent volume for SQLite + media files |
| `socket-server` | Fly.io (shared-cpu-1x, 256MB) | Native WebSocket support, always-on |

All three Fly.io apps fit exactly within the **free tier** (3 shared-cpu-1x machines always-on).

---

## Code Changes

### `blog/backend/src/main.rs`
- Made `DATABASE_URL` configurable via environment variable
- Defaults to `sqlite:data/blog.db` if not set
- Enables easy future migration to PostgreSQL (e.g. Neon) by just changing the secret

```rust
let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/blog.db".to_string());
```

### `blog/backend/src/infrastructure/web/server/mod.rs`
- Added `sqlx::migrate!().run(&pool).await?` after pool creation
- Embeds all migration files from `migrations/` into the binary at compile time
- Replaces the manual `sqlx migrate run` CLI step from the old deploy script
- Also switched from `SqlitePool::connect()` to `SqlitePool::connect_with()` with `.create_if_missing(true)` — fixes a `SQLITE_CANTOPEN` (error 14) crash on fresh Fly.io volumes where the DB file does not yet exist

### `blog/backend/src/infrastructure/web/api/router.rs`
- CORS allowed origin now reads from `ALLOWED_ORIGIN` env var
- Falls back to `https://portfolio.huuthangle.site` if not set
- No code change needed when domain changes in future

### `blog/backend/example.env`
- Fully documented all environment variables including new ones (`DATABASE_URL`, `ALLOWED_ORIGIN`, JWT expiry hours, SMTP config)

---

## New Files

### `blog/backend/fly.toml`
- Mounts a persistent volume `blog_data` at `/app/data`
- Both the SQLite DB (`/app/data/blog.db`) and media files (`/app/data/media`) live on the same volume
- `MEDIA_PATH=/app/data/media` set in `[env]`
- `min_machines_running = 1` — always-on, no cold starts

### `blog/frontend/fly.toml`
- `PORT=8080` set in `[env]` (read by `svelte-adapter-bun`)
- `API_URL` set as a secret (see issues section)
- `min_machines_running = 1`

### `socket-server/Dockerfile`
- The socket server had no Dockerfile at all
- Simple `node:20-alpine` image, copies `server.js` and `package*.json`, runs `node server.js`

### `socket-server/fly.toml`
- Fly.io's HTTP proxy handles WebSocket `Upgrade` headers natively — no special config needed

### `.github/workflows/deploy.yml` (replaced)
- Removed all `appleboy/ssh-action` VPS SSH steps
- Removed portfolio deploy job (Cloudflare Pages handles this automatically on push)
- Added `socket-server/**` to path filter and `deploy-socket-server` job
- Split `deploy-blog` into `deploy-blog-backend` + `deploy-blog-frontend` (frontend has `needs: deploy-blog-backend` to ensure ordering)
- All deploy steps use `superfly/flyctl-actions/setup-flyctl@master` + `flyctl deploy --remote-only --config <path>`
- Single `FLY_API_TOKEN` secret replaces `VPS_HOST`, `VPS_USERNAME`, `VPS_SSH_KEY`, `GH_PAT`

---

## Deployment Steps Performed

```bash
# Backend
fly apps create my-blog-backend
fly volumes create blog_data --region sin --size 3
fly secrets set JWT_SECRET="..." ALLOWED_ORIGIN="https://portfolio.huuthangle.site"
fly deploy  # from blog/backend/

# Frontend
fly apps create my-blog-frontend
fly secrets set API_URL="https://my-blog-backend.fly.dev"
fly deploy  # from blog/frontend/

# Certs
fly certs add blog.huuthangle.site --app my-blog-frontend

# GitHub Actions token
fly tokens create deploy -x 999999h
# → added as FLY_API_TOKEN secret in GitHub repo settings
```

---

## Issues Encountered & Resolved

### 1. `SQLITE_CANTOPEN` (error 14) on fresh volume

**Symptom:** Backend crashed in a restart loop immediately after first deploy.  
```
Connecting to sqlite:data/blog.db
Error: Database(SqliteError { code: 14, message: "unable to open database file" })
Machine has reached its max restart count of 10
```

**Root cause:** `sqlx::SqlitePool::connect()` uses `create_if_missing = false` by default. On the VPS the DB file always pre-existed (created by `sqlx migrate run` in the old deploy script). On a fresh Fly.io volume there is nothing, so SQLite returned `CANTOPEN`.

**Fix:** Switched to `connect_with()` + `.create_if_missing(true)`:
```rust
let pool = sqlx::SqlitePool::connect_with(
    db_url.parse::<sqlx::sqlite::SqliteConnectOptions>()?
        .create_if_missing(true),
).await?;
```

---

### 2. Fly.io internal networking (`*.internal`) does not work with Bun

**Symptom:** Frontend returned HTTP 500 on every page. Backend was healthy.

**Diagnosis:** `API_URL` was initially set to `http://my-blog-backend.internal:3000` (Fly.io's private IPv6 WireGuard network). DNS resolved correctly (`fdaa:64:fb15:a7b:6b7:f5d8:e6d7:2`) but Bun's HTTP client could not establish a socket connection to the IPv6 address.

```bash
# DNS resolves fine:
node -e "dns.lookup('my-blog-backend.internal', {family: 6}, (e,a) => console.log(e,a))"
# → null fdaa:64:fb15:a7b:6b7:f5d8:e6d7:2

# But Bun fetch fails:
bun -e "fetch('http://my-blog-backend.internal:3000/posts/latest').then(...).catch(e => console.error(e.message))"
# → Unable to connect. Is the computer able to access the url?

# Public URL works fine:
bun -e "fetch('https://my-blog-backend.fly.dev/posts/latest').then(r=>r.text()).then(console.log)"
# → {"featured_posts":[]}
```

**Fix:** Changed `API_URL` secret to the public Fly.io hostname:
```bash
fly secrets set API_URL="https://my-blog-backend.fly.dev" --app my-blog-frontend
```

---

### 3. Fly.io auto-created a second frontend machine

**Symptom:** After first `fly deploy` on the frontend, two machines were running (consuming 2 of the 3 free-tier slots).

**Root cause:** Fly.io automatically creates a second machine on first deployment for high-availability and zero-downtime deploys.

**Fix:**
```bash
fly scale count 1 --app my-blog-frontend --yes
```

---

### 4. `blog.huuthangle.site` DNS not resolving to Fly.io (cert not verified)

**Symptom:** `fly certs check blog.huuthangle.site` reported "Not verified — DNS records do not match".

**Root cause:** The A/AAAA records in Cloudflare DNS were set with the **orange cloud (proxied)** mode. Cloudflare intercepts requests before they reach Fly.io, preventing Let's Encrypt from completing the HTTP-01 or DNS-01 challenge.

**Fix (Option A — simplest):** In Cloudflare DNS, switch the A and AAAA records for `blog.huuthangle.site` to **DNS only** (grey cloud). Fly.io then handles TLS directly.

**Fix (Option B — keep Cloudflare proxy):** Add the following DNS records and set SSL mode to **Full** in Cloudflare:
```
TXT   _fly-ownership.blog.huuthangle.site  →  app-123r8qo
CNAME _acme-challenge.blog.huuthangle.site →  blog.huuthangle.site.123r8qo.flydns.net
```

---

### 5. `fly sftp shell put` silently refuses to overwrite existing files

**Symptom:** After fixing the `SQLITE_CANTOPEN` crash, the backend started with an empty database (0 posts, 0 users, 0 media) despite the user having uploaded `blog.db` via `fly sftp shell`.

**Root cause:** The `fly sftp shell` `put` command outputs `file exists on VM` and **refuses to overwrite** when the destination file already exists. The `create_if_missing` fix had created a fresh empty DB before the SFTP upload was attempted, so the upload was silently rejected.

**Diagnosis:**
```bash
fly ssh console --app my-blog-backend -C "sqlite3 /app/data/blog.db 'SELECT COUNT(*) FROM posts'"
# → 0
```

**Fix:** Delete the empty DB first via SSH, then re-upload:
```bash
# 1. Remove the empty blocking file
fly ssh console --app my-blog-backend -C "rm /app/data/blog.db"

# 2. Upload the real database (from blog/backend/)
fly sftp shell --app my-blog-backend
# → put data/blog.db /app/data/blog.db

# 3. Restart to reconnect
fly apps restart my-blog-backend

# 4. Verify
fly ssh console --app my-blog-backend -C "sqlite3 /app/data/blog.db 'SELECT COUNT(*) FROM posts'"
```

---

## Free Tier Usage (post-migration)

| Resource | Used | Free allowance |
|---|---|---|
| Fly.io shared-cpu-1x machines | 3 (`my-blog-backend`, `my-blog-frontend`, `my-ws-server`) | 3 |
| Fly.io persistent volume | 3 GB (`blog_data` on `my-blog-backend`) | 3 GB |
| Cloudflare Pages | 1 project (`portfolio`) | Unlimited |

**Monthly cost: $0**

---

## DNS Records Summary (Cloudflare)

| Type | Name | Value | Proxy |
|---|---|---|---|
| A | `portfolio` | Cloudflare Pages (auto-managed) | ✅ Proxied |
| A | `blog` | `66.241.124.143` | ⚠️ DNS only (grey cloud) |
| AAAA | `blog` | `2a09:8280:1::fc:3b4e:0` | ⚠️ DNS only (grey cloud) |
| A | `ws` | `<ws-app-ip>` | ⚠️ DNS only (grey cloud) |

---

## Remaining TODOs

- [ ] Verify `blog.huuthangle.site` cert is issued after DNS is set to grey cloud
- [ ] Deploy and test `socket-server` (`my-ws-server`)
- [ ] Add `socket-server` to `fly certs` + DNS for `ws.huuthangle.site`
- [ ] Decommission the old VPS once all services are confirmed live
- [ ] Consider adding [Litestream](https://litestream.io/) for continuous SQLite replication to Cloudflare R2 as a backup strategy
- [ ] Update the footer text in `blog/frontend` — it still says "Running on a VPS"
```

Now let me commit it: