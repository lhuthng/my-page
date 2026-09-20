# Backend Architecture

Audience: backend contributors. Update trigger: a change to the layer model,
module boundaries, or the file budget.

The backend lives in `blog/backend/src` and is organized layer-first,
aggregate-second: three layers, each split by the same aggregate axis
(post, project, game, v86, audiobook, media, series, user, auth, dashboard,
newsletter), so a feature is findable by name in every layer.

## Layers and dependency rules

```text
src/
|-- domain/          # Entities, value objects, domain errors - no sqlx, no axum
|-- application/     # Command structs + service port traits - no I/O
`-- infrastructure/  # Axum handlers, SQLite adapters, storage, mail, sync, GraphQL
```

| Module | May depend on | Must never depend on |
| --- | --- | --- |
| `domain` | `std`, `serde`, `chrono`, `validator`, `uuid` | `application`, `infrastructure`, `sqlx`, `axum` |
| `application::commands` | `domain`, `std` | `sqlx`, `axum`, `infrastructure` |
| `application::services` | `domain`, `application::commands` | `sqlx`, `axum`, `infrastructure` |
| `infrastructure::persistence` | `application`, `domain`, `sqlx` | `infrastructure::web`, `axum` |
| `infrastructure::storage` | `domain` | `infrastructure::web`, `infrastructure::persistence` |
| `infrastructure::mail` | `domain` | `infrastructure::web` |
| `infrastructure::sync` | `domain`, `infrastructure::storage`, `sqlx` | `infrastructure::web` |
| `web::server` | everything (composition root) | — |
| `web::api::support` | `domain`, `axum`, `application` | any `handlers::<feature>` module |
| `web::api::handlers::<feature>` | `domain`, `application`, `web::server::AppState`, `support`, `middlewares` | `persistence` directly, `graphql`, any *other* `handlers::<feature>` |
| `web::api::router` | every `handlers::<feature>::routes` | business logic |
| `web::graphql` | `domain`, `web::server::AppState`, `sqlx` (named exception, see below) | `web::api::handlers` |

Named exception: GraphQL reads the `SqlitePool` directly instead of going
through the service traits. It is a read-only admin surface and the direct
access is deliberate — see
[../decisions/0001-backend-modularization.md](../decisions/0001-backend-modularization.md)
(§3.4, Option A).

## Key seams

- **Ports and adapters.** `application::services` defines one port trait per
  aggregate (`PostService`, `GameService`, ...). `infrastructure::persistence`
  implements each against SQLite. `AppState` composes one service per
  aggregate; handlers reach services only through it. Handlers never touch
  `persistence` directly and never import each other — cross-feature handler
  utilities live in `web::api::support` (one concern per file, generic over
  the aggregate's error type).
- **Route ownership.** Every feature module exposes a `routes(state)` function
  holding its route table, auth middleware stack, and body limits.
  `web::api::router.rs` only nests the features and applies the global
  CORS/compression/trace policy (`web::api::layers`). Route tables that live
  under another feature's prefix (v86 endpoints under `/games`, newsletter
  management under `/dashboard`) are mounted by that feature's `routes()` but
  defined by the owning feature.
- **Shared code.** Helpers used by two or more feature modules belong in
  `web::api::support`, never in a second copy.

## File budget

| Kind of file | Target | Hard ceiling |
| --- | --- | --- |
| Production module | 150–300 | 400 |
| `mod.rs` (wiring, re-exports, `routes()`) | 40–150 | 200 |
| `dto.rs` / `rows.rs` | 100–300 | 400 / 350 |
| `router.rs` (composer) | ≤ 80 | 120 |
| Test module (`tests.rs`) | — | exempt |

When a module crosses its target it is split along use case (`read.rs`,
`write.rs` first; a third file only when a coherent sub-responsibility
emerges). Unit tests live in a sibling `<module>/tests.rs`, declared as
`#[cfg(test)] mod tests;`.

The full design rationale, target tree, and implementation deviations are
recorded in
[../decisions/0001-backend-modularization.md](../decisions/0001-backend-modularization.md).
