# Blog Frontend

SvelteKit frontend for the blog, using
[`svelte-adapter-bun`](https://github.com/gornostay25/svelte-adapter-bun) for
SSR. Bun is both the runtime and the package manager. Runs at
[huuthangle.site](https://huuthangle.site).

## Overview

The app is fully server-side rendered — no static export; it runs as a
persistent Bun process. In production it is containerized (`FROM oven/bun:1`)
and sits behind nginx on the same VM as the backend. The request paths and
the nginx topology are documented in
[../../docs/architecture/frontend.md](../../docs/architecture/frontend.md)
and [../../docs/architecture/deployment.md](../../docs/architecture/deployment.md).

## Environment variables

Documented once in [../../docs/guides/configuration.md](../../docs/guides/configuration.md)
(`API_URL`, `BACKEND_ORIGIN`, `ALLOWED_HOSTS`, `TRUSTED_ORIGINS`, `PORT`,
`BODY_SIZE_LIMIT`). See `example.env` for a starting point.

## Local development

```bash
bun install
bun dev      # backend must be running at http://localhost:3000
```

Or the full stack with Docker Compose from `blog/`:
`docker compose up -d --build` — frontend at `http://localhost:5000`.

## Building

```bash
bun run build
```

Output goes to `build/`; the entry point is `build/index.js` (the Docker
image's `ENTRYPOINT`).
