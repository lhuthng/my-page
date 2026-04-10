# Blog Frontend

SvelteKit frontend for the blog, using [`svelte-adapter-bun`](https://github.com/gornostay25/svelte-adapter-bun) for SSR. Bun is both the runtime and the package manager. Runs at [blog.huuthangle.site](https://blog.huuthangle.site).

## Overview

The app is fully server-side rendered. There is no static export — it runs as a persistent Bun process. In production it's containerized (`FROM oven/bun:1`) and sits behind nginx on the same VM as the backend.

## Request Flow

Three distinct paths depending on what's being requested:

```
1. Server-side data fetching (hooks.server.js, +page.server.js)
   └── route() in $lib/server/proxy.js
         └── prepends API_URL → http://backend:3000 (Docker internal, never public)

2. Browser API calls
   └── fetch('/api/<path>')
         └── src/routes/api/[...path]/+server.js
               └── proxyFallback() → API_URL (backend)

3. Media files
   └── fixClientRoute() in $lib/server/proxy.js
         ├── BACKEND_ORIGIN set → direct browser fetch from that origin
         │     (nginx routes /media/* straight to the backend, skipping SvelteKit)
         └── BACKEND_ORIGIN unset → /api/media/... (proxy fallback)
```

**Why the split for media?** When `BACKEND_ORIGIN` is set, the browser fetches images directly from the backend origin. This means one fewer network hop, no memory pressure on the frontend container, and the backend's `Cache-Control` headers reach the browser unmodified.

## Environment Variables

| Variable | Description |
|---|---|
| `API_URL` | Backend URL for server-side calls. In Docker: `http://backend:3000`. Locally: `http://localhost:3000`. Never exposed to the browser. |
| `BACKEND_ORIGIN` | Public origin of the backend (e.g. `https://blog.huuthangle.site`). Tells `fixClientRoute()` to build direct browser-facing media URLs. Optional — omit it and media falls back to the `/api/media/...` proxy. |
| `PORT` | Port the SvelteKit server listens on. Set to `8080` in `docker-compose.yml`. |

See `example.env` for a starting point.

## Local Development

Run the frontend standalone (you'll need the backend running separately at `http://localhost:3000`):

```bash
bun install
bun dev
```

Or spin up the full stack with Docker Compose from the `blog/` directory:

```bash
docker compose up -d --build
```

Frontend will be available at `http://localhost:5000`.

## Building

```bash
bun run build
```

Output goes to `build/`. The entry point is `build/index.js` — run it directly with `bun run build/index.js`. The Docker image does exactly this via its `ENTRYPOINT`.