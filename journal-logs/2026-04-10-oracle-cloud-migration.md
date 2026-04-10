# Migration Journal: Fly.io → Oracle Cloud Free Tier
**Date:** 2026-04-10  
**Author:** Huu Thang

---

## Context

Fly.io quietly ended their free tier. The three apps that were running free (`my-blog-backend`, `my-blog-frontend`, `my-ws-server`) started accruing charges. The goal was to migrate `my-blog-backend` and `my-blog-frontend` to a permanently free alternative with zero monthly cost — without changing domain names or breaking functionality.

`my-ws-server` (the WebSocket proxy) was left on Fly.io as-is since it was out of scope.

### Services Being Migrated

| Service | Tech | Old hosting |
|---|---|---|
| `blog/frontend` | SvelteKit SSR, `svelte-adapter-bun`, Docker | Fly.io `shared-cpu-1x` 256MB |
| `blog/backend` | Rust/Axum + SQLite + local media folder, Docker | Fly.io `shared-cpu-1x` 256MB + 3GB volume |

---

## Target Architecture

| Layer | Technology | Why |
|---|---|---|
| VM | Oracle Cloud Always Free (AMD micro, 1 OCPU, 1GB RAM) | Truly free forever, not a trial |
| Containers | Docker Compose | Same as before, minimal changes |
| Reverse proxy | nginx + Certbot/Let's Encrypt | Same as before, already in repo |
| CI/CD | GitHub Actions → GHCR → SSH deploy | VM too weak to compile Rust; build on GitHub's servers instead |
| Image registry | GitHub Container Registry (GHCR) | Free for public packages, native GitHub integration |

### Intended vs Actual VM

The plan was to use **VM.Standard.A1.Flex** (ARM, 4 OCPU / 24GB RAM — always free). The instance was created as **VM.Standard.E2.1.Micro** (AMD x86, 1 OCPU / 1GB RAM) by mistake. A1.Flex was unavailable in the region at the time of migration, so the AMD micro was kept.

This had significant consequences (see Issues section).

---

## Final Architecture on the VM

```
Browser request
    │
    ├── /media/...  ──────────────────────────────────────────────────┐
    │   (images, uploads)                                               │
    │                                               nginx → :3001 → rust-backend
    │                                               (skips SvelteKit, saves RAM)
    │
    └── everything else  ─────────────────────────┐
        (pages, /api/..., graphql)                  │
                                   nginx → :5000 → svelte-frontend
                                                          │
                                          server-side SSR/hooks calls
                                                          │
                                             http://backend:3000
                                             (Docker internal network)
                                                   rust-backend
```

- **Port 5000** — SvelteKit frontend, bound to `127.0.0.1` (nginx only)
- **Port 3001** — Rust backend, bound to `127.0.0.1` (nginx only, for media)
- **Port 3000** — Rust backend inside Docker network (SvelteKit server-side calls only)
- **Port 80/443** — nginx, public

---

## Code Changes

### `.github/workflows/deploy.yml` (replaced entirely)

Old workflow used `flyctl deploy --remote-only` for all services. New workflow:

- `build-push-backend` — runs on GitHub's `ubuntu-latest` (7GB RAM), builds the Rust Docker image for `linux/amd64`, pushes to `ghcr.io/lhuthng/blog-backend:latest`
- `build-push-frontend` — same for the SvelteKit image, pushes to `ghcr.io/lhuthng/blog-frontend:latest`
- Both jobs run **in parallel** and use `cache-from/cache-to: type=gha` — unchanged Rust dependencies are restored from cache in seconds on subsequent pushes
- `deploy-blog` — runs after both build jobs succeed; uses `appleboy/ssh-action` to SSH into the Oracle VM, runs `docker compose pull && docker compose up -d --remove-orphans`
- `deploy-socket-server` — unchanged, still uses `flyctl deploy` for `my-ws-server`

New GitHub secrets required:

| Secret | Value |
|---|---|
| `VM_HOST` | Oracle VM public IP |
| `VM_USER` | `ubuntu` |
| `VM_SSH_KEY` | Private key contents for VM SSH access |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` scope (VM pulls images) |
| `FLY_API_TOKEN` | Unchanged (socket-server still on Fly.io) |

### `blog/docker-compose.yml`

- Added `image: ghcr.io/lhuthng/blog-backend:latest` and `image: ghcr.io/lhuthng/blog-frontend:latest` — `docker compose pull` now fetches pre-built images from GHCR instead of building locally
- Added `ports: ["127.0.0.1:3001:3000"]` to the backend service — exposes the backend on a host-local port so nginx can proxy `/media/` directly without going through SvelteKit

### `.github/nginx/blog.conf`

- Fixed long-standing `server_name` typo in the 443 block (`blog.huuthang.site` → `blog.huuthangle.site`)
- Added `location /media/` block that proxies directly to `http://localhost:3001/media/` — browser fetches media straight from the backend via nginx, no SvelteKit memory overhead
- Removed hardcoded Fly.io-era SSL cert paths (now managed by Certbot on the VM)

### `blog/backend/Dockerfile` (VM-only temporary change)

During the first attempted build on the VM, added `CARGO_BUILD_JOBS=1` to `cargo chef cook` and `cargo build` steps to limit memory usage. This change was **never committed** — it was applied directly on the VM and became irrelevant once the CI/CD pipeline took over building.

---

## Data Migration from Fly.io

```bash
# Download SQLite database
flyctl ssh sftp get /app/data/blog.db -a my-blog-backend
# → 448 KB

# Archive media directory on the Fly.io machine
flyctl ssh console -a my-blog-backend -C "tar -czf /tmp/media.tar.gz -C /app/data media"

# Download the archive
flyctl ssh sftp get /tmp/media.tar.gz -a my-blog-backend
# → ~14 MB, 75 media subdirectories

# Upload both to Oracle VM
scp blog.db ubuntu@<VM_IP>:~/MyPage/blog/backend/data/blog.db
scp media.tar.gz ubuntu@<VM_IP>:~/
ssh ubuntu@<VM_IP> "tar -xzf ~/media.tar.gz -C ~/MyPage/blog/backend/"
```

---

## VM Setup Steps Performed

```bash
# Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker ubuntu
sudo apt install docker-compose-plugin -y

# nginx + Certbot
sudo apt install nginx certbot python3-certbot-nginx git rsync -y

# 4GB swap (critical — see Issues)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Firewall (see Issues for the iptables ordering problem)
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo netfilter-persistent save

# Code (private repo — git clone over HTTPS fails without token)
rsync -avz -e "ssh -i <key>" \
  --exclude='blog/backend/target' --exclude='blog/frontend/node_modules' \
  --exclude='blog/frontend/build' --exclude='.git' \
  blog .github ubuntu@<VM_IP>:~/MyPage/

# SSL
sudo certbot --nginx -d blog.huuthangle.site -d huuthangle.site \
  --non-interactive --agree-tos -m <email>
```

---

## Issues Encountered & Resolved

### 1. Wrong VM shape created — no A1.Flex available

**Symptom:** `free -h` on the VM showed 954MB RAM. `nproc` showed 2. `uname -m` showed `x86_64`.

**Root cause:** The instance was created as `VM.Standard.E2.1.Micro` (AMD, 1GB RAM) instead of `VM.Standard.A1.Flex` (ARM, up to 4 OCPU / 24GB RAM). The AMD micro shape cannot be resized. A1.Flex was unavailable in the tenancy at the time.

**Impact:** Only 1GB RAM, which is insufficient for compiling Rust (see Issue 2). Resolved by moving compilation off the VM entirely.

---

### 2. VM froze and became unreachable during Rust compilation (OOM)

**Symptom:** SSH connections timed out. `ping` got 100% packet loss. The VM had to be hard-rebooted from the Oracle Cloud console.

**Root cause:** `docker compose build` with a Rust project of this size (`axum`, `sqlx`, `async-graphql`, `tokio`, etc.) requires several GB of RAM. The VM had 1GB RAM and no swap, so the OOM killer took out critical processes, freezing the entire machine.

**First fix attempt:** Added 4GB swap + `CARGO_BUILD_JOBS=1` in the Dockerfile — correct approach but abandoned in favour of a better solution (see below).

**Final fix:** Killed the on-VM build entirely. Moved all image building to GitHub Actions (`ubuntu-latest` runner has ~7GB RAM and 2 fast CPUs). The VM now only runs `docker compose pull` — it never compiles anything.

```bash
# Kill the runaway build
kill -9 <docker-compose-pid> <docker-buildx-pid>
sudo kill -9 <rustc-pid>
```

---

### 3. iptables REJECT rule blocked ports 80 and 443

**Symptom:** `curl http://blog.huuthangle.site/` returned `Connection refused`. Certbot could not complete the HTTP-01 ACME challenge.

**Root cause:** Oracle Cloud's Ubuntu image ships with a default iptables INPUT chain that has a catch-all `REJECT` rule. The script originally ran `sudo iptables -I INPUT 6 ...` to insert ACCEPT rules for ports 80 and 443, but the REJECT rule was at position 5 — so all traffic matched REJECT before reaching the ACCEPT rules.

```
# Broken state (REJECT fires first):
5    REJECT     all  --  0.0.0.0/0   0.0.0.0/0   reject-with icmp-host-prohibited
6    ACCEPT     tcp  --  0.0.0.0/0   0.0.0.0/0   state NEW tcp dpt:443
7    ACCEPT     tcp  --  0.0.0.0/0   0.0.0.0/0   state NEW tcp dpt:80
```

**Fix:** Delete the misplaced rules and re-insert before the REJECT:

```bash
sudo iptables -D INPUT -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -D INPUT -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo netfilter-persistent save

# Correct state:
4    ACCEPT     tcp  --  0.0.0.0/0   0.0.0.0/0   state NEW tcp dpt:22
5    ACCEPT     tcp  --  0.0.0.0/0   0.0.0.0/0   state NEW tcp dpt:80
6    ACCEPT     tcp  --  0.0.0.0/0   0.0.0.0/0   state NEW tcp dpt:443
7    REJECT     all  --  0.0.0.0/0   0.0.0.0/0   reject-with icmp-host-prohibited
```

**Lesson:** On Oracle Cloud Ubuntu images, always insert iptables rules at position **≤ 5** (before the REJECT), or check the existing chain with `sudo iptables -L INPUT -n --line-numbers` first.

---

### 4. Private GitHub repo could not be cloned on the VM

**Symptom:** `git clone https://github.com/lhuthng/my-page.git` failed with `fatal: could not read Username`.

**Root cause:** The repo is private. HTTPS clone requires credentials; SSH clone requires a deploy key. Neither was configured on the VM.

**Fix:** Used `rsync` over SSH from the local machine directly to the VM, excluding build artifacts and `node_modules`. Faster than setting up a deploy key for a one-time operation.

---

### 5. Media files returning 404 after migration

**Symptom:** Images on the blog were broken. Backend logs showed no requests for media at all.

**Root cause:** `BACKEND_ORIGIN=https://blog.huuthangle.site` was set in the frontend `.env`. The `fixClientRoute()` function in `proxy.js` uses this to build browser-facing media URLs:

```js
// With BACKEND_ORIGIN set:
fixClientRoute("media/i/my-slug")
// → "https://blog.huuthangle.site/media/i/my-slug"
```

On Fly.io this worked because `BACKEND_ORIGIN` pointed to the backend's own public domain (`my-blog-backend.fly.dev`). On Oracle Cloud, the backend has no public domain — nginx only routes traffic to the SvelteKit frontend at port 5000. There was no nginx rule for `/media/`, so these URLs hit SvelteKit and got 404s.

**Fix:** Expose the backend on an internal host port and add a dedicated nginx location block:

```yaml
# docker-compose.yml
backend:
  ports:
    - "127.0.0.1:3001:3000"   # internal only, nginx can reach it
```

```nginx
# nginx blog.conf
location /media/ {
    proxy_pass http://localhost:3001/media/;
}
```

`BACKEND_ORIGIN` stays as `https://blog.huuthangle.site`. The browser now fetches `https://blog.huuthangle.site/media/...`, nginx proxies it straight to the Rust backend on port 3001 — skipping SvelteKit entirely. This is important on a 1GB RAM machine where avoiding unnecessary memory allocation in the frontend container matters.

---

## Free Tier Usage (post-migration)

| Resource | Used | Free allowance |
|---|---|---|
| Oracle Cloud VM (`VM.Standard.E2.1.Micro`) | 1 (blog backend + frontend) | 2 always-free AMD micros |
| Oracle Cloud block storage | ~6GB (OS + swap + data) | 200GB total |
| GHCR image storage | ~675MB (backend 151MB + frontend 524MB) | Free for public packages |
| GitHub Actions minutes | ~10 min per blog push | 2,000 min/month (free tier) |
| Fly.io (`my-ws-server`) | 1 machine | Paid (out of scope) |

**Monthly cost for blog: $0**

---

## DNS Records Summary (post-migration)

| Type | Name | Content | Proxy |
|---|---|---|---|
| A | `blog` | `141.147.45.193` | ⚪ DNS only |
| A | `huuthangle.site` | `141.147.45.193` | ⚪ DNS only |
| A | `dev` | `141.147.45.193` | ⚪ DNS only |
| A | `ws` | `66.241.124.10` | ⚪ DNS only |

DNS-only (grey cloud) is required because TLS is terminated directly on the VM by Certbot/nginx. Cloudflare proxy would intercept the ACME HTTP-01 challenge and prevent certificate issuance.

---

## Remaining TODOs

- [ ] Try creating `VM.Standard.A1.Flex` (ARM, 4 OCPU / 24GB) again when availability allows — migrate to it for better compile headroom and more memory for containers
- [ ] Update the footer text in `blog/frontend` — it still references Fly.io
- [ ] Consider [Litestream](https://litestream.io/) for continuous SQLite replication to object storage as a backup strategy
- [ ] Automate the nginx config update step in CI (currently a manual scp if changed)
- [ ] Verify `docker compose up -d` restart policy is set so containers come back after VM reboots