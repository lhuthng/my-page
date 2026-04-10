# my-page

Personal website monorepo — [Huu Thang](https://github.com/lhuthng) · [github.com/lhuthng/my-page](https://github.com/lhuthng/my-page)

Three independently deployed services living in one repo: a blog, a portfolio, and a WebSocket server.

---

## Services

| Service | Domain | Stack | Hosting |
|---|---|---|---|
| Blog | [blog.huuthangle.site](https://blog.huuthangle.site) | SvelteKit SSR + Rust/Axum + SQLite | Oracle Cloud VM (Docker Compose) |
| Portfolio | [portfolio.huuthangle.site](https://portfolio.huuthangle.site) | React 19 + Tailwind + GSAP + Lottie, static | Cloudflare Pages |
| WebSocket server | [ws.huuthangle.site](https://ws.huuthangle.site) | Node.js `ws` library | Fly.io |

---

## Repo Structure

```
my-page/
├── .github/
│   ├── workflows/deploy.yml   # CI/CD pipeline
│   └── nginx/                 # nginx config files for all services
├── blog/
│   ├── backend/               # Rust/Axum REST + GraphQL API
│   ├── frontend/              # SvelteKit SSR app
│   └── docker-compose.yml
├── portfolio/                 # React static site
├── socket-server/             # Node.js WebSocket server
└── journal-logs/              # Migration and ops journals
```

---

## Architecture

```
Browser
  │
  └─→ Cloudflare DNS
        │
        ├─→ blog.huuthangle.site
        │     └─→ Oracle Cloud VM
        │           └─→ nginx
        │                 ├─→ SvelteKit frontend (:5000)
        │                 │     └─→ backend (Docker internal, :3000)
        │                 └─→ backend media (:3001)
        │
        ├─→ portfolio.huuthangle.site
        │     └─→ Cloudflare Pages CDN (static)
        │
        └─→ ws.huuthangle.site
              └─→ Fly.io
                    └─→ Node.js WebSocket server (:5001)
```

The blog runs entirely on the Oracle VM. nginx handles TLS termination and proxies to the SvelteKit container; SvelteKit makes server-side requests to the Rust backend over the Docker internal network, so the backend is never directly exposed to the public internet. The portfolio is fully static — no server involved at runtime. The WebSocket server is a lightweight standalone process on Fly.io.

---

## CI/CD

Push to `master` triggers the [GitHub Actions pipeline](.github/workflows/deploy.yml). Path filters make sure only the relevant jobs run.

| Changed path | What happens |
|---|---|
| `blog/**` | Docker images built on GitHub's servers, pushed to GHCR, then SSH'd onto the Oracle VM to pull and restart containers |
| `socket-server/**` | `flyctl deploy` to Fly.io |
| `portfolio/**` | Cloudflare Pages detects the push and deploys automatically — no workflow step needed |

You can also force a blog or socket deploy regardless of changed files by including `[deploy blog]` or `[deploy ws]` in your commit message.

Required GitHub secrets:

| Secret | Purpose |
|---|---|
| `VM_HOST` | Oracle VM public IP |
| `VM_USER` | SSH username (`ubuntu`) |
| `VM_SSH_KEY` | Private key for the `github-actions` keypair |
| `GHCR_TOKEN` | GitHub PAT with `read:packages` (so the VM can pull images) |
| `FLY_API_TOKEN` | Fly.io deploy token |

See [`.github/README.md`](.github/README.md) for full infrastructure setup instructions.

---

## Local Development

### Blog

Requires `.env` files — copy from the provided examples first:

```bash
cp blog/backend/example.env blog/backend/.env
cp blog/frontend/example.env blog/frontend/.env
```

Then bring everything up with Docker Compose:

```bash
cd blog && docker compose up -d
```

### Portfolio

```bash
cd portfolio && bun install && bun dev
```

### Socket server

```bash
cd socket-server && node server.js
```

---

## Documentation

- [`blog/README.md`](blog/README.md) — blog architecture, API, environment variables
- [`portfolio/README.md`](portfolio/README.md) — portfolio build and deployment
- [`socket-server/README.md`](socket-server/README.md) — WebSocket server protocol and config
- [`.github/README.md`](.github/README.md) — infrastructure setup and CI/CD
- [`journal-logs/`](journal-logs/) — migration notes and ops history