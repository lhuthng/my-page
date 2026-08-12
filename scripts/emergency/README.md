# Manual Deploy (local build)

A deploy path that builds and pushes images from this machine instead of
GitHub Actions — used when CI is unavailable/broken, or for any push tagged
`[manual deploy]` (see `.github/README.md`), which skips the CI pipeline for
that commit. The VPS is too weak to build, so images are built here and
pushed to GHCR, then the VPS pulls and reloads them.

This is intentional and manual on purpose:

- `frontend` (Bun/SvelteKit): cheap to build (no native compile) — use this
  to ship static/SEO changes quickly.
- `backend` (Rust): cross-compiled for `linux/amd64` via `docker buildx`
  (QEMU emulation). Slow; only when Rust changes.

## One-time setup (do once, yourself)

1. Start the local Docker daemon (Docker Desktop / OrbStack / colima).
2. Log into GHCR: `docker login ghcr.io` (use a PAT with `write:packages`).
3. Copy `deploy.env.example` to `deploy.env`, fill in `VM_HOST`, `VM_USER`,
   `VM_SSH_KEY`, and confirm you can `ssh` to the VPS with that key.
   `deploy.env` is gitignored — never commit it.

## Use

```bash
scripts/emergency/deploy.sh --check        # verify prereqs only
scripts/emergency/deploy.sh                # deploy frontend (default)
scripts/emergency/deploy.sh frontend
scripts/emergency/deploy.sh backend
scripts/emergency/deploy.sh all            # backend + frontend
```

Each deploy: `docker buildx build --platform linux/amd64 --push` the image,
then on the VPS `docker compose pull && up -d <svc> && image prune -f`.

## Smoke test

The script curls `/robots.txt` and `/BingSiteAuth.xml`. If `/BingSiteAuth.xml`
shows 404 while `/robots.txt` shows 200, purge that URL in Cloudflare (stale
edge 404) — otherwise the new static file should be reachable.

## Note

This is a real, ongoing part of the deploy workflow now — not only a
fallback for CI outages — so it's not going away once CI is healthy.