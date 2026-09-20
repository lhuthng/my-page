# Frontend Architecture

Audience: frontend contributors. Update trigger: a change to the request
paths, the adapter, or the SSR setup.

The frontend is a fully server-side rendered SvelteKit 2 app running as a
persistent Bun process (`svelte-adapter-bun`); there is no static export. In
production it is containerized (`FROM oven/bun:1`) behind nginx.

## Structure

- `src/routes` — page routes, `+page.server.js` loads, and the `/api/[...path]`
  proxy route.
- `src/lib/server/proxy.js` — `route()` (server-side API calls) and
  `fixClientRoute()` (browser media URLs).
- `src/lib/components` — UI building blocks, including the editor shell,
  dashboard widgets, and the audiobook player.
- `src/lib/api` — typed wrappers around the REST surface.

## Request flow

```text
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

The media split keeps one network hop off the frontend container and lets the
backend's `Cache-Control` headers reach the browser unmodified.

## Environment

See [../guides/configuration.md](../guides/configuration.md) for `API_URL`,
`BACKEND_ORIGIN`, `ALLOWED_HOSTS`, `TRUSTED_ORIGINS`, `PORT`, and
`BODY_SIZE_LIMIT`. Build output goes to `build/` with entry point
`build/index.js`.
