# Media and Storage

Audience: backend contributors and operators. Update trigger: a storage
layout or serving change.

## Media files

Uploaded media (images, video, audio, models, Lottie) always lives on local
disk under `MEDIA_PATH`, content-addressed by hash + file type + uploader.
Audio is served with HTTP Range requests (`206 Partial Content`), which is
what makes audiobook streaming seek instantly without loading a chapter into
memory.

## ObjectStore (v86 game artifacts)

v86 artifacts — system IMG chunks, game disks, launcher ISOs, snapshots,
saves — go through the `ObjectStore` abstraction in
`src/infrastructure/storage/`. HTML5/WebGL demo ZIPs and js-dos bundles
always live on local disk regardless of this setting.

| `STORAGE_BACKEND` | Behavior |
| --- | --- |
| `auto` (default) | Cloudflare R2 when the `R2_*` variables are set, otherwise the local filesystem |
| `r2` | Require R2; startup fails if the `R2_*` variables are incomplete |
| `fs` | Local filesystem under `PROJECT_DEMOS_PATH` |

- In `fs` mode artifacts land at `{PROJECT_DEMOS_PATH}/{storage_key}` — the
  same key layout the R2 mirror uses, so the backends are interchangeable.
- In `fs` mode the backend serves artifacts itself at relative URLs
  (`/v86/snapshots/...`, `/games/s/{slug}/v86/...`); `R2_PUBLIC_URL` is
  ignored, and nginx must route the artifact prefixes to the backend.
- In `r2` mode the descriptor points straight at the R2 public domain and the
  browser fetches chunks from the CDN.

Storage keys are content-addressed (sha256-derived), which makes the R2→fs
backfill idempotent — see [../guides/operations.md](../guides/operations.md).

Env vars: [../guides/configuration.md](../guides/configuration.md).
