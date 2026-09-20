# Data Model

Audience: backend contributors touching the schema. Update trigger: a new
migration or a change to the aggregate/table mapping.

## Storage

SQLite (sqlx 0.8), single file at `blog/backend/data/blog.db` by default.
Migrations live in `blog/backend/migrations` (54 at the time of writing) and
run automatically on backend startup; `make migrate` runs them manually.

Migration policy:

- Migrations are forward-only and run inside startup, so a failed migration
  prevents the backend container from becoming healthy — the deploy workflow
  prints backend logs to make that visible.
- Table-rebuild migrations (e.g. relaxing a UNIQUE constraint) rely on the
  dedicated migration pool started with `foreign_keys(false)`; SQLite ignores
  PRAGMA foreign_keys inside a transaction, so FK checks must be off at
  connect time for those migrations.

## Aggregates and tables

| Aggregate | Primary tables | Notes |
| --- | --- | --- |
| Post | `posts`, `post_stats`, `tags`, `post_tags`, `comments` | Draft/published/trash lifecycle; `content_kind` distinguishes game/project posts |
| Project | `projects` | `delegate_game_id` lets a project play a game's launcher; demo type html5/webgl/embed/download/video/none |
| Game | `games`, `game_jsdos_bundles`, `game_v86_games`, `game_v86_variants`, `game_v86_saves`, `game_v86_snapshots`, upload-session tables | `artifact_revision` drives optimistic concurrency on v86 artifacts |
| v86 systems | `v86_systems`, `v86_system_versions` | Content-addressed storage keys |
| Audiobook | `audiobooks`, `audiobook_tracks`, `audiobook_tags` | Tag vocabulary separate from global `tags` |
| Media | `media`, `media_aliases` | Content-addressed files under `MEDIA_PATH` |
| User/Auth | `users`, `user_meta`, `sessions`, verification/reset tables | Role tiers: user, moderator, admin |
| Newsletter | `newsletter_subscribers`, `newsletter_campaigns` | `sync_keys.mode` only allows `'pull'` |
| Analytics | analytics tables | View/like beacons and visitor countries |

Authoritative shape: the `migrations/` directory is the source of truth; this
table is a map, not a schema dump.
