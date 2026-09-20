# Development

Audience: contributors working on the blog day to day. Update trigger: a new
or changed Makefile target, migration policy, or gate.

## Make targets

Run from `blog/`.

| Command | What it does |
| --- | --- |
| `make setup` | Creates the data directory and copies missing example env files. |
| `make backend` | Runs the Rust backend locally with Cargo. |
| `make frontend` | Runs the SvelteKit dev server with Bun. |
| `make migrate` | Runs sqlx migrations manually. |
| `make docker-up` | Starts Docker Compose services in the background. |
| `make docker-down` | Stops Docker Compose services. |
| `make docker-build` | Rebuilds Docker images locally. |
| `make lint` | Runs backend clippy with warnings denied and frontend Prettier check. |
| `make fmt` | Runs Cargo fmt and frontend Prettier write. |

Manual migrations default to `sqlite://data/blog.db` and can be overridden:

```bash
make migrate DB_URL=sqlite://path/to/blog.db
```

## Migrations

Migrations live in `blog/backend/migrations` and run automatically on backend
startup. Create one with `sqlx migrate add <name>`; the filename date is
ISO 8601. See [../architecture/data-model.md](../architecture/data-model.md)
for the policy (FK-off migration pool, startup semantics).

## Tests

- Backend unit tests: `cargo test` from `blog/backend`; per-module tests live
  in sibling `tests.rs` files (see
  [../architecture/backend.md](../architecture/backend.md)).
- Backend integration tests: `blog/backend/tests/` (post/audiobook
  persistence, migrations, validation).
- Frontend: focused tests for comment syntax and thread behavior alongside
  the components.

## Gates

- `cargo fmt` and `cargo clippy -- -D warnings` for the backend.
- Prettier for the frontend.
- Commit conventions and the documentation rules: [../../CONTRIBUTING.md](../../CONTRIBUTING.md).
