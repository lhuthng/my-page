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

Runs on GitHub's servers, cross-compiles the Go binary for `linux/amd64`, SCPs it to the Oracle VM, then SSHes in to replace the binary and restart the systemd service:

```bash
# on the VM
sudo mv /tmp/socket-server/server-linux /usr/local/bin/socket-server
sudo chmod +x /usr/local/bin/socket-server
sudo systemctl restart socket-server
```

The server runs as a systemd unit (`socket-server.service`) with `Restart=on-failure`. nginx sits in front on ports 80/443 and proxies WebSocket connections to `localhost:5001`.

Live at: `wss://wss.huuthangle.site/ws`

---

## Required GitHub Secrets

| Secret | Description |
|---|---|
| `VM_HOST` | Oracle Cloud VM public IP (blog server) |
| `VM_USER` | SSH username (`ubuntu`) |
| `VM_SSH_KEY` | Private SSH key contents for blog VM access |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` scope — used by the VM to pull images from GHCR |
| `WS_VM_HOST` | Oracle Cloud VM public IP (socket server — `130.61.121.230`) |
| `WS_VM_USER` | SSH username (`ubuntu`) |
| `WS_VM_SSH_KEY` | Private SSH key contents for socket server VM access |

`GITHUB_TOKEN` is used automatically by the build jobs to push images; no secret needed for that.

`FLY_API_TOKEN` is no longer required — the socket server moved from Fly.io to Oracle Cloud.

---

## Nginx Configs

Reference configs live in `.github/nginx/`. These are kept in sync with what's deployed on the Oracle VM. To apply changes manually, copy to `/etc/nginx/sites-available/` and symlink to `sites-enabled/`.

| File | Domain | Behavior |
|---|---|---|
| `blog.conf` | `blog.huuthangle.site` | `/media/*` → `backend:3001`, everything else → `frontend:5000` |
| `root.conf` | `huuthangle.site` | 301 redirect to `blog.huuthangle.site` |
| `ws.conf` | `wss.huuthangle.site` | WebSocket proxy (`/ws`) + health check (`/`) → `localhost:5001`; TLS via Let's Encrypt |
| `portfolio.conf` | `portfolio.huuthangle.site` | Static files from `/var/www/portfolio` — legacy VPS config, kept for reference |

`portfolio.conf` is no longer active — the portfolio is on Cloudflare Pages.

Add a rate limiter in `/etc/nginx/nginx.conf`:

```nginx
http {
    limit_req_zone $binary_remote_addr zone=blog:10m rate=5r/s;
    # ... rest of config unchanged ...
}
```

Then reference it in `blog.conf` as needed.

---

## Oracle Cloud VM — First-Time Setup (Blog VM)

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
sudo apt install -y nginx rsync
sudo snap install --classic certbot
sudo ln -sf /snap/bin/certbot /usr/bin/certbot
```

**4. Add swap** (critical on the AMD micro — Rust compilation will OOM without it)

```bash
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

**5. Fix iptables**

Oracle's default iptables has a blanket REJECT rule. Insert ACCEPT rules for ports 80 and 443 **before** it:

```bash
sudo iptables -I INPUT -p tcp --dport 80  -j ACCEPT
sudo iptables -I INPUT -p tcp --dport 443 -j ACCEPT
sudo netfilter-persistent save
```

**6. Set up nginx**

```bash
sudo cp .github/nginx/blog.conf /etc/nginx/sites-available/blog
sudo cp .github/nginx/root.conf /etc/nginx/sites-available/root
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

---

## Oracle Cloud VM — First-Time Setup (Socket Server VM)

**1. Create the VM** — same Oracle Cloud Console steps as above.

**2. Open ports in OCI Security List** — TCP 80 and 443.

**3. SSH in and install nginx + certbot**

```bash
sudo apt update && sudo apt install -y nginx
sudo snap install --classic certbot
sudo ln -sf /snap/bin/certbot /usr/bin/certbot
```

**4. Fix iptables**

```bash
sudo iptables -I INPUT -p tcp --dport 80  -j ACCEPT
sudo iptables -I INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -I INPUT -p tcp --dport 5001 -j ACCEPT
sudo netfilter-persistent save
```

**5. Deploy the binary and systemd service**

```bash
# cross-compile on your machine
GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o server-linux .
scp -i ~/.ssh/ssh-key-support-server.key server-linux ubuntu@<ip>:/tmp/socket-server
ssh -i ~/.ssh/ssh-key-support-server.key ubuntu@<ip> "
  sudo mv /tmp/socket-server /usr/local/bin/socket-server
  sudo chmod +x /usr/local/bin/socket-server
"
```

Create `/etc/systemd/system/socket-server.service`:

```ini
[Unit]
Description=Go WebSocket relay server
After=network.target

[Service]
Type=simple
User=ubuntu
ExecStart=/usr/local/bin/socket-server
Restart=on-failure
RestartSec=5
Environment=PORT=5001
Environment=MAX_ROOM_SIZE=4
StandardOutput=journal
StandardError=journal
SyslogIdentifier=socket-server

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now socket-server
```

**6. Set up nginx and TLS**

```bash
sudo cp .github/nginx/ws.conf /etc/nginx/sites-available/socket-server
sudo ln -s /etc/nginx/sites-available/socket-server /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d wss.huuthangle.site
```

From here, GitHub Actions handles all future deploys automatically.