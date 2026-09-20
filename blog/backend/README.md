# Blog Backend

Rust 2024 edition · Axum 0.8 · SQLite via sqlx 0.8 · Port 3000

## What this is

The blog's REST API, admin GraphQL surface, media and artifact storage, and
the `sync-pull` tool. Layer-first architecture split by aggregate — the
module map, dependency contract, and file budget live in
[../../docs/architecture/backend.md](../../docs/architecture/backend.md).

## Run it

```bash
cp example.env .env    # then fill in values
make backend           # from blog/, or: cargo run --bin backend
```

Migrations run automatically on startup. Manual management:

```bash
sqlx migrate add <migration_name>
sqlx migrate run --database-url sqlite://data/blog.db
```

## Configure it

Every environment variable is documented once in
[../../docs/guides/configuration.md](../../docs/guides/configuration.md).
Email (SMTP or Brevo) is optional — the server starts without it.

## Where things live

| Path | Contents |
| --- | --- |
| `src/domain/` | Entities, value objects, per-aggregate error enums |
| `src/application/` | Command structs + service port traits |
| `src/infrastructure/persistence/` | SQLite adapters implementing the traits |
| `src/infrastructure/web/api/` | Feature handlers (`routes()` per feature), support utilities, router |
| `src/infrastructure/web/graphql/` | Admin GraphQL (read-only) |
| `src/infrastructure/web/server/` | AppState, config, lifecycle, maintenance jobs |
| `src/infrastructure/storage/` | ObjectStore (fs / R2) for v86 artifacts |
| `src/infrastructure/mail/` | Transports and branded templates |
| `src/infrastructure/sync/` | prod → dev pull protocol |
| `src/bin/sync-pull/` | The pull tool |
| `migrations/` | sqlx migrations |

## Storage modes

v86 artifacts go through the `ObjectStore` abstraction (`auto`/`r2`/`fs`,
see `STORAGE_BACKEND`); media and demo ZIPs always live on disk. Key layout,
serving behavior, and the R2↔fs switch:
[../../docs/reference/media-and-storage.md](../../docs/reference/media-and-storage.md)
and [../../docs/guides/operations.md](../../docs/guides/operations.md).

## API

Full REST surface: [../../docs/reference/api-rest.md](../../docs/reference/api-rest.md).
GraphQL (`/graphql`, moderator+, read-only):
[../../docs/reference/api-graphql.md](../../docs/reference/api-graphql.md).

## Backup and sync

Sync keys, the `sync-pull` tool, VM backup cron, and restore:
[../../docs/guides/operations.md](../../docs/guides/operations.md).
