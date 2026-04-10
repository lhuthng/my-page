# CI/CD & Infrastructure

## Pipeline Overview

Defined in `.github/workflows/deploy.yml`. Triggers on push to `master`, but only runs jobs relevant to what actually changed — a `dorny/paths-filter` step gates each job on its path.

You can also force a deploy regardless of changed paths by including `[deploy blog]` or `[deploy ws]` in your commit message.

---

## Blog Deployment

Triggered by changes in `blog/`.

### `build-push-backend` and `build-push-frontend`

These two jobs run in parallel on GitHub's servers. Each builds a Docker image for `linux/amd64` and pushes it to GHCR:

- `ghcr.io/lhuthng/blog-backend:latest`
- `ghcr.io/lhuthng/blog-frontend:latest`

Both use `cache-from: type=gha` / `cache-to: type=gha,mode=max`, so unchanged Rust deps and npm packages are restored from GitHub Actions cache instead of being rebuilt.

### `deploy-blog`

Runs after both build jobs succeed. SSHes into the Oracle VM and runs:

```bash
docker login ghcr.io
docker compose pull
docker compose up -d --remove-orphans
docker image prune -f
```

---

## Socket Server Deployment

Triggered by changes in `socket-server/`.

### `deploy-socket-server`

Runs `flyctl deploy --remote-only` from the `socket-server/` directory. Fly.io handles the build remotely — no local Docker required.

---

## Required GitHub Secrets

| Secret | Description |
|---|---|
| `VM_HOST` | Oracle Cloud VM public IP |
| `VM_USER` | SSH username (`ubuntu`) |
| `VM_SSH_KEY` | Private SSH key contents for VM access |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` scope — used by the VM to pull images from GHCR |
| `FLY_API_TOKEN` | Fly.io deploy token — used for socket server deployment |

`GITHUB_TOKEN` is used automatically by the build jobs to push images; no secret needed for that.

---

## Nginx Configs

Reference configs live in `nginx/`. Copy to `/etc/nginx/sites-available/` and symlink to `sites-enabled/`.

| File | Domain | Behavior |
|---|---|---|
| `blog.conf` | `blog.huuthangle.site` | `/media/*` → `backend:3001`, everything else → `frontend:5000` |
| `root.conf` | `huuthangle.site` | 301 redirect to `blog.huuthangle.site` |
| `ws.conf` | `ws.huuthangle.site` | WebSocket proxy to `localhost:5001` — legacy VPS config, kept for reference |
| `portfolio.conf` | `portfolio.huuthangle.site` | Static files from `/var/www/portfolio` — legacy VPS config, kept for reference |

`ws.conf` and `portfolio.conf` are no longer active. The socket server is on Fly.io and the portfolio is on Cloudflare Pages — neither goes through nginx anymore.

Add a rate limiter in `/etc/nginx/nginx.conf`:

```nginx
http {
    limit_req_zone $binary_remote_addr zone=blog:10m rate=5r/s;
    # ... rest of config unchanged ...
}
```

Then reference it in `blog.conf` as needed.

---

## Oracle Cloud VM — First-Time Setup

See the full migration journal: `journal-logs/2026-04-10-oracle-cloud-migration.md`

Quick reference:

**1. Create the VM**

Use `VM.Standard.E2.1.Micro` (free tier AMD) or `VM.Standard.A1.Flex` (free tier ARM, better) in the Oracle Cloud Console.

**2. Open ports in OCI Security List**

Add ingress rules for TCP ports 80 and 443.

**3. SSH in and install dependencies**

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo apt install -y nginx certbot python3-certbot-nginx rsync
```

**4. Add swap** (critical on the AMD micro — Rust compilation will OOM without it)

```bash
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

**5. Fix iptables**

Oracle's default iptables has a blanket REJECT rule. Insert ACCEPT rules for ports 80 and 443 **before** it (position 5):

```bash
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 5 -m state --state NEW -p tcp --dport 443 -j ACCEPT
```

**6. Set up nginx**

```bash
sudo cp nginx/blog.conf /etc/nginx/sites-available/blog
sudo cp nginx/root.conf /etc/nginx/sites-available/root
sudo ln -s /etc/nginx/sites-available/blog /etc/nginx/sites-enabled/
sudo ln -s /etc/nginx/sites-available/root /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

**7. TLS with certbot**

Make sure DNS is pointed at the VM before running this.

```bash
sudo certbot --nginx -d blog.huuthangle.site -d huuthangle.site
```

**8. Clone the repo and start the stack**

```bash
git clone https://github.com/lhuthng/MyPage.git ~/MyPage
cd ~/MyPage/blog
docker compose up -d
```

From here, GitHub Actions handles all future deploys automatically.