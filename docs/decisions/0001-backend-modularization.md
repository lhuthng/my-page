# Backend Modularization and Documentation Architecture

Status: **accepted 2026-09-20** — Part A (backend modularization) implemented
starting at commit `92825bbe`; Part B (documentation) follows. The §3.4 open
decision is resolved below.

## §3.4 decision record — GraphQL's database access (2026-09-20)

**Option A — accept and document the exception.** `web::graphql` keeps direct
`SqlitePool` access. GraphQL is a read-only admin surface; the resolvers are
shaped by the rows they select, and routing them through the service ports
would add a reshaping pass with no boundary gain. The boundary contract in
§3.4 records this as a named exception: `web::graphql` may use `sqlx` directly
in addition to `domain` and `web::server::AppState`.

## Implementation deviations (Part A, 2026-09-20)

The target tables in Part A were refined during implementation where they
conflicted with the file budget (§3.5) or with what the code actually
contains. All deviations are recorded in the commit messages; the material
ones:

- **handlers/game** and **handlers/post**: the single `write.rs` the tables
  allocate would hold 515 and 570 lines respectively, so the write side is
  split along use case per §3.5's own rule (game: `write`/`update`/`trash`/
  `publish`; post: `write`/`update`/`publish`/`trash`).
- **handlers/v86**: the table's `systems.rs`/`games.rs`/`snapshots.rs` would
  each exceed 400 lines, so the upload pipelines were split one level further
  (`system_uploads`, `system_versions`, `game_uploads`, `snapshot_uploads`,
  `snapshot_finalize`) and two shared modules added (`shared.rs` for the
  cross-file helpers, `upload_session.rs` for the multipart relay).
- **persistence/post**: the trait impl is a single block in `mod.rs` whose
  methods delegate to `pub(super)` inherent methods in `read`/`detail`/`write`/
  `comments`/`threads`. (The table's single read.rs/write.rs/comments.rs would
  all exceed 400 lines.) A first cut that gave each file its own
  `impl PostService for PostServiceImpl` block does not compile: Rust forbids
  multiple trait-impl blocks for the same trait-and-type pair (E0119), so the
  ADR's earlier claim that "Rust permits split impls" was wrong and is
  retracted. Inherent candidates win over trait candidates in method
  resolution, so the delegation is a one-line body per method. `mapping.rs`
  carries `into_snapshot` and the hydration helper; the row structs are
  re-exported from `mod.rs`. The same one-impl-per-adapter pattern applies to
  every split persistence adapter (audiobook, auth, dashboard, game, media,
  newsletter, project, series).
- **persistence/media**: `change_post_cover` (303 lines) is its own
  `covers.rs`; avatar and upload are separate files. Eleven files instead of
  the table's seven.
- **persistence/series**: no `rows.rs` — the adapter reads via `query_as`
  tuples, not `FromRow` structs.
- **persistence/project**: no `links.rs` — the link-table SQL is interleaved
  with the upserts inside the write methods, unlike post's free functions.
- **persistence/dashboard**: the table's nine files are consolidated to six
  (`shared.rs` carries the snapshot fetchers reused by every listing).
- **graphql**: `types.rs` and `rows.rs` stay whole — both are within budget
  and the per-aggregate split would create fourteen files of 20-40 lines.
- **server/config/cors.rs**: not created; allowed-origin resolution is
  HTTP-layer policy and lives in `api/layers.rs` (§4.4's one-fact-one-home).
- **sync**: no `snapshot.rs` — this module contains no snapshot code; the
  SQLite snapshot is streamed by the `/sync/database` handler.
- **Open item — domain error enums still touch `axum`/`sqlx`**: the eleven
  per-aggregate error enums implement `IntoResponse` and `From<sqlx::Error>`
  directly in `domain/errors/`, so the acceptance criterion "domain compiles
  with `sqlx` and `axum` removed" is **not yet met**. Moving those impls into
  the web/persistence layers rewrites every error conversion path in the
  crate and was deliberately deferred until it can be done with compiler
  feedback. Relatedly, `handlers/dashboard.rs` response type references
  `persistence::analytics::VisitorCountryStat` (a type leak, pre-existing).
- **domain/ and application/**: the flat per-aggregate files are already the
  layout the §3.3 tree describes (one file per aggregate, all under budget);
  converting them to `post/mod.rs`-style directories would change no path and
  no file size, so the flat layout is kept.
Scope: `blog/backend/` (module architecture) and every tracked `*.md` file in the repository (documentation architecture).

---

## 1. Purpose

The backend has grown into 106 Rust files totalling ~31,400 lines, but that code is
not evenly distributed. Seven files hold 42% of it and thirteen more hold another
29%. Those twenty files are the entire maintenance problem: they are the files
nobody can hold in their head, the files where a change to one feature risks an
unrelated one, and the files where the same helper has been copy-pasted three
times instead of shared.

This document defines the **target** structure and architecture that fixes that —
the end state, not the route to it. It specifies:

- the target directory tree for `blog/backend/src/`
- the module boundary contract and the dependency rules that enforce it
- a file-size budget that keeps the problem from recurring
- the shared support layer that absorbs today's duplicated helpers
- the target allocation of every oversized module into cohesive sub-files
- the target documentation tree for the whole repository

This document deliberately contains **no implementation steps and no code
changes**. It answers "what should exist and what are the rules", not "how do we
get there".

---

## 2. Constraints and Non-Goals

**Constraints that shape the target**

| Constraint | Consequence for the design |
| --- | --- |
| The existing `domain → application → infrastructure` layering is a real dependency boundary | Keep it. Split *inside* each layer, do not replace the layers. |
| `AppState` composes one service per aggregate and handlers reach services through it | Service traits and their names stay stable; only the files behind them change. |
| `application::services` holds the port traits; `infrastructure::persistence` holds the adapters | This ports-and-adapters seam is the anchor of the whole design and is preserved verbatim. |
| SQLite + sqlx with `FromRow` row structs | Row structs get their own file per aggregate rather than living beside business rules. |
| Rust module conventions in this repo already use `mod.rs` | Keep `mod.rs` for directories; do not mix in the `foo.rs` + `foo/` sibling style. |

**Explicit non-goals**

- No behaviour changes, no API shape changes, no route renames, no schema changes.
- No switch from layer-first to full vertical-slice (feature-first) layout — see §3.2.
- No framework migration, no rewrite of the persistence strategy.
- No restructuring of `portfolio/`, `blog/frontend/src/`, `tmp/`, `target/`, or `.zcode/`.
- No CI/CD or tooling changes. Enforcement policy is stated; wiring it up is out of scope.

---

# Part A — Target Backend Architecture

## 3.1 Design principles

1. **One module, one responsibility.** A module that needs the word "and" to
   describe it is two modules.
2. **A file is a unit of comprehension, not a unit of convenience.** Target size
   is 150–300 lines. The hard ceiling is 400 for production code.
3. **Wiring is not logic.** `mod.rs` files declare modules and re-export; they do
   not contain business rules, SQL, or handlers.
4. **Shared behaviour lives in one place.** Any helper used by two or more
   feature modules belongs in `web/api/support/`, not in a second copy.
5. **Dependencies point inward.** `domain` knows nothing. `application` knows
   `domain`. `infrastructure` knows both. Never the reverse.
6. **Aggregates are the natural seam.** The repository already has a consistent
   aggregate vocabulary (post, project, game, v86, audiobook, media, series, user,
   auth, dashboard, newsletter). Every layer is organised along that same axis so a
   feature can be located by name in every layer.

## 3.2 Layer-first, aggregate-second — and why

The two candidate layouts are:

| Layout | Shape | Verdict |
| --- | --- | --- |
| **Layer-first** (chosen) | `domain/` → `application/` → `infrastructure/`, each split by aggregate | **Adopt.** The layers are enforced by the compiler today: `application::services` defines the traits, `infrastructure::persistence` implements them, `AppState` wires them. Splitting each layer by aggregate removes the file-length problem without touching a single architectural boundary. |
| Vertical slice (rejected) | `features/post/{domain,application,http}/` | **Reject.** It would dissolve the ports-and-adapters seam into eleven parallel seams, force every aggregate to re-declare its own trait placement convention, and make the `AppState` composition root the only place the architecture is still visible. It solves a discoverability problem the repository does not have. |

The chosen layout keeps the macro-architecture intact and makes the **aggregate
the second axis in all three layers**, so `post` is findable at the same relative
position in `domain/`, `application/`, `infrastructure/persistence/`, and
`infrastructure/web/api/handlers/`.

## 3.3 Target directory tree

```text
blog/backend/src/
├── main.rs                          # entry point: logging + server bootstrap
├── lib.rs                           # module roots; public surface (unchanged)
│
├── domain/                          # pure types — no sqlx, no axum, no I/O
│   ├── mod.rs
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── shared/                  # value_objects.rs, pagination.rs, ids.rs
│   │   ├── post/                    # post.rs, comment.rs, tag.rs, series.rs
│   │   ├── project/                 # project.rs, demo.rs
│   │   ├── game/                    # game.rs, v86.rs
│   │   ├── audiobook/               # audiobook.rs, track.rs
│   │   ├── media/                   # media.rs, alias.rs
│   │   ├── user/                    # user.rs, auth.rs, secret.rs
│   │   ├── dashboard/               # overview.rs, growth.rs, trash.rs
│   │   ├── newsletter/              # subscriber.rs, campaign.rs
│   │   └── mail.rs
│   └── errors/
│       ├── mod.rs
│       ├── shared.rs                # cross-aggregate errors only
│       └── <one file per aggregate> # post.rs, project.rs, game.rs, v86.rs, ...
│
├── application/                     # use-case contracts only — no I/O
│   ├── mod.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── post/                    # post.rs, comment.rs, tag.rs, series.rs
│   │   ├── project/                 # project.rs, demo.rs
│   │   ├── game/                    # game.rs, v86.rs
│   │   ├── audiobook/               # audiobook.rs, track.rs
│   │   ├── media.rs
│   │   ├── user.rs
│   │   ├── auth.rs
│   │   ├── dashboard.rs
│   │   └── newsletter.rs
│   └── services/                    # port traits — one per aggregate, unchanged
│       ├── mod.rs
│       └── post.rs project.rs game.rs audiobook.rs media.rs series.rs
│           user.rs auth.rs dashboard.rs newsletter.rs
│
├── infrastructure/
│   ├── mod.rs
│   │
│   ├── persistence/                 # adapter layer — implements application::services
│   │   ├── mod.rs
│   │   ├── post/                    # mod.rs rows.rs mapping.rs links.rs
│   │   │                            # read.rs write.rs comments.rs tags.rs tests.rs
│   │   ├── project/                 # mod.rs rows.rs mapping.rs read.rs write.rs links.rs
│   │   ├── game/                    # mod.rs rows.rs mapping.rs read.rs write.rs
│   │   ├── audiobook/               # mod.rs rows.rs mapping.rs read.rs write.rs
│   │   │                            # tracks.rs tags.rs validation.rs medium.rs
│   │   ├── media/                   # mod.rs rows.rs hashing.rs aliases.rs crud.rs
│   │   │                            # search.rs files.rs
│   │   ├── series/                  # mod.rs rows.rs read.rs write.rs
│   │   ├── user/                    # mod.rs rows.rs read.rs write.rs
│   │   ├── auth/                    # mod.rs rows.rs tokens.rs sessions.rs credentials.rs
│   │   ├── dashboard/               # mod.rs rows.rs overview.rs posts.rs projects.rs
│   │   │                            # users.rs tags.rs trash.rs analytics.rs
│   │   ├── newsletter/              # mod.rs rows.rs subscribers.rs campaigns.rs
│   │   ├── analytics.rs
│   │   └── image_convert.rs
│   │
│   ├── storage/                     # object store — leaf, no web deps
│   │   ├── mod.rs                   # ObjectStore enum + dispatch
│   │   ├── keys.rs                  # storage-key layout helpers
│   │   ├── fs.rs
│   │   └── r2.rs
│   │
│   ├── mail/                        # leaf, no web deps
│   │   ├── mod.rs                   # transport selection
│   │   ├── smtp.rs
│   │   ├── brevo.rs
│   │   ├── messages/                # one sender per message family
│   │   │   ├── mod.rs
│   │   │   ├── contact.rs
│   │   │   ├── verification.rs
│   │   │   ├── password_reset.rs
│   │   │   └── newsletter.rs
│   │   └── templates/               # templates.rs split by family
│   │       ├── mod.rs
│   │       ├── layout.rs
│   │       ├── auth.rs
│   │       └── newsletter.rs
│   │
│   ├── sync/                        # prod → dev pull protocol
│   │   ├── mod.rs                   # public surface
│   │   ├── manifest.rs              # manifest assembly
│   │   ├── snapshot.rs              # consistent SQLite snapshot
│   │   ├── files.rs                 # media / demo / artifact transfer
│   │   └── rewrite.rs               # path rewriting for target layout
│   │
│   └── web/
│       ├── mod.rs
│       │
│       ├── server/                  # composition root + runtime
│       │   ├── mod.rs               # re-exports only
│       │   ├── state.rs             # AppState, AppConfig, artifact_base_url()
│       │   ├── config/              # one file per config concern
│       │   │   ├── mod.rs
│       │   │   ├── database.rs
│       │   │   ├── auth.rs
│       │   │   ├── media.rs
│       │   │   ├── project_demo.rs
│       │   │   ├── storage.rs
│       │   │   ├── mail.rs
│       │   │   └── cors.rs
│       │   ├── lifecycle.rs         # HTTPServer, startup, shutdown
│       │   └── maintenance/         # background jobs, one file each
│       │       ├── mod.rs
│       │       ├── upload_sessions.rs   # orphaned upload cleanup
│       │       ├── trash.rs             # expired trash purge
│       │       ├── game_artifacts.rs    # artifact GC
│       │       └── reading_times.rs     # reading-time backfill
│       │
│       ├── api/
│       │   ├── mod.rs
│       │   ├── router.rs            # THIN composer only (~60 lines)
│       │   ├── layers.rs            # CORS + compression + trace policy
│       │   │
│       │   ├── support/             # shared handler utilities (see §3.6)
│       │   │   ├── mod.rs
│       │   │   ├── multipart.rs
│       │   │   ├── media_short_names.rs
│       │   │   ├── demo_archive.rs
│       │   │   ├── links.rs
│       │   │   ├── ownership.rs
│       │   │   ├── slug.rs
│       │   │   └── cover.rs
│       │   │
│       │   ├── middlewares/
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs
│       │   │   └── analytics.rs
│       │   ├── secrets/
│       │   │   └── mod.rs
│       │   │
│       │   └── handlers/            # one DIRECTORY per feature
│       │       ├── mod.rs
│       │       ├── post/            # mod.rs dto.rs read.rs write.rs
│       │       │                    # comments.rs tags.rs media.rs response.rs
│       │       ├── project/         # mod.rs dto.rs read.rs write.rs demo.rs response.rs
│       │       ├── game/            # mod.rs dto.rs read.rs write.rs jsdos.rs response.rs
│       │       ├── v86/             # mod.rs dto.rs constants.rs manifest.rs
│       │       │                    # systems.rs games.rs snapshots.rs runtime.rs
│       │       │                    # serving.rs saves.rs tests.rs
│       │       ├── audiobook/       # mod.rs dto.rs read.rs write.rs tracks.rs response.rs
│       │       ├── media/           # mod.rs dto.rs serving.rs upload.rs manage.rs aliases.rs
│       │       ├── series/          # mod.rs dto.rs
│       │       ├── user/            # mod.rs dto.rs profile.rs avatar.rs
│       │       ├── auth/            # mod.rs dto.rs session.rs password.rs verification.rs
│       │       ├── dashboard/       # mod.rs dto.rs overview.rs content.rs users.rs
│       │       ├── newsletter/      # mod.rs dto.rs
│       │       ├── mail/            # mod.rs dto.rs
│       │       └── sync/            # mod.rs dto.rs
│       │
│       └── graphql/
│           ├── mod.rs               # composition + public surface
│           ├── schema.rs            # BlogSchema, build_schema
│           ├── query/               # one file per domain, merged into QueryRoot
│           │   ├── mod.rs
│           │   ├── users.rs posts.rs comments.rs media.rs series.rs
│           │   ├── taxonomy.rs      # tags + categories
│           │   ├── stats.rs         # db_stats, overview
│           │   ├── dashboard.rs     # dashboard_posts, dashboard_projects
│           │   ├── featured.rs      # featured_posts, featured_projects
│           │   ├── detail.rs        # post_detail, project_detail, related_posts
│           │   └── slug.rs          # check_slug, check_project_slug
│           ├── types/               # one file per aggregate
│           │   ├── mod.rs
│           │   ├── user.rs post.rs comment.rs media.rs series.rs
│           │   ├── taxonomy.rs dashboard.rs
│           └── rows/                # one file per aggregate
│               ├── mod.rs
│               └── user.rs post.rs comment.rs media.rs series.rs
│                   taxonomy.rs dashboard.rs
│
└── bin/
    ├── encode.rs                    # unchanged (13 lines)
    └── sync-pull/                   # becomes a package
        ├── main.rs                  # entry point only
        ├── cli.rs                   # argument parsing
        ├── plan.rs                  # manifest → transfer plan
        ├── download.rs              # transfers
        ├── rewrite.rs               # local path rewriting
        └── tests.rs
```

## 3.4 Module boundary contract

These rules are normative. A dependency that violates them is a defect, not a
style preference.

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
| `web::api::support` | `domain`, `axum`, `application` | any specific `handlers::<feature>` module |
| `web::api::handlers::<feature>` | `domain`, `application`, `web::server::AppState`, `web::api::support`, `web::api::middlewares` | `infrastructure::persistence` directly, `web::graphql`, any *other* `handlers::<feature>` |
| `web::api::router` | every `handlers::<feature>::routes` | business logic of any kind |
| `web::graphql` | `domain`, `web::server::AppState` | `infrastructure::web::api::handlers` — **named exception (Option A, 2026-09-20): graphql may use `sqlx` directly against the pool** |
| `helper` | `std` | everything else |

Three consequences worth stating explicitly:

- **Handlers never reach into the database.** They call `state.<aggregate>_service`,
  which is the port trait. `infrastructure::persistence` stays invisible to the
  HTTP layer, exactly as it is today.
- **Handlers never import each other.** Cross-feature behaviour goes through
  `support/` or through a service. This is what removes the current coupling risk
  in the post/project/game handler trio.
- **`support/` may not import a feature module.** Otherwise it becomes a
  back-channel between features and the boundary above is meaningless.

### Open decision: GraphQL's database access

`web::graphql` currently issues SQL directly against the `SqlitePool` instead of
going through the service traits. The target architecture must pick one:

- **Option A — accept and document the exception.** GraphQL is a read-only admin
  surface; direct pool access is a deliberate performance choice. The boundary
  table above records it as an explicit, named exception.
- **Option B — route through services.** GraphQL resolvers call
  `AppState` services like REST handlers do. Uniform boundaries, at the cost of
  reshaping several resolvers.

The rest of this document assumes **Option A**, because it is the smaller change
and the exception is already load-bearing. This is called out so it is chosen
rather than inherited by accident.

## 3.5 File budget policy

| Kind of file | Target | Hard ceiling |
| --- | --- | --- |
| Production module (`read.rs`, `write.rs`, `manifest.rs`, …) | 150–300 | 400 |
| `mod.rs` (wiring, re-exports, `routes()`) | 40–150 | 200 |
| `dto.rs` (request/response types) | 100–300 | 400 |
| `router.rs` (composer) | ≤ 80 | 120 |
| `rows.rs` (sqlx `FromRow` structs) | 100–250 | 350 |
| Test module (`tests.rs`) | — | exempt |
| `main.rs` / `lib.rs` | ≤ 50 | 60 |

Supporting rules:

- A `mod.rs` that contains a `fn` with a SQL string, a business `match`, or an
  Axum handler is a mis-split module.
- When a module crosses its target it is split along **use case**, not along
  "first half / second half". `read.rs` and `write.rs` are the default first cut;
  a third file appears only when a coherent sub-responsibility emerges (e.g.
  `tracks.rs` inside `audiobook/`).
- Inline `#[cfg(test)] mod tests` blocks move to a sibling `tests.rs` declared as
  `#[cfg(test)] mod tests;`. Eleven files currently carry inline test blocks; the
  largest single block is 135 lines.

## 3.6 Shared support layer — the de-duplication map

This is the single highest-value change in the plan. The post, project, and game
handler modules currently carry near-identical copies of the same utilities. The
project and game modules are the worst case: every one of the following exists
twice, once in `handlers/project.rs` and once in `handlers/game.rs`.

| Duplicated symbol | Current locations | Target home |
| --- | --- | --- |
| `parse_project_multipart` / `parse_game_multipart` | `handlers/project.rs:173`, `handlers/game.rs:176` | `support/multipart.rs` — one generic parser |
| `upload_inline_media` | `handlers/project.rs:273`, `handlers/game.rs:276` | `support/multipart.rs` |
| `replace_media_short_names` | `handlers/project.rs:326`, `handlers/game.rs:329`, `handlers/post.rs:669` | `support/media_short_names.rs` |
| `extract_media_short_names` | `handlers/post.rs:653` | `support/media_short_names.rs` |
| `MEDIA_NAME_REGEXES` | `handlers/project.rs:319`, `handlers/game.rs:322`, `handlers/post.rs:646` | `support/media_short_names.rs` |
| `ShortNameExtraction` | `handlers/project.rs:159`, `handlers/game.rs:162`, `handlers/post.rs:639` | `support/media_short_names.rs` |
| `has_invalid_component` | `handlers/project.rs:351`, `handlers/game.rs:354` | `support/demo_archive.rs` |
| `normalized_zip_path` | `handlers/project.rs:360`, `handlers/game.rs:363` | `support/demo_archive.rs` |
| `strip_common_root` | `handlers/project.rs:377`, `handlers/game.rs:380` | `support/demo_archive.rs` |
| `extract_demo_zip` / `extract_demo_zip_blocking` | `handlers/project.rs:405/416`, `handlers/game.rs:408/419` | `support/demo_archive.rs` |
| `normalize_links` | `handlers/project.rs:574`, `handlers/game.rs:854` | `support/links.rs` |
| `validate_demo_url` | `handlers/project.rs:600`, `handlers/game.rs:915` | `support/links.rs` |
| `require_project_owner` / `require_game_owner` | `handlers/project.rs:549`, `handlers/game.rs:619` | `support/ownership.rs` |
| `is_admin_or_mod` / `is_admin_or_mod_game` | `handlers/project.rs:780`, `handlers/game.rs:1101` | `support/ownership.rs` |
| `require_can_delete_project` / `require_can_delete_game` | `handlers/project.rs:784`, `handlers/game.rs:1105` | `support/ownership.rs` |
| Cover-upload plumbing | `handlers/common.rs` (whole file) | `support/cover.rs` |
| Check-slug request/response pairs | `post.rs`, `project.rs`, `game.rs`, `audiobook.rs` | `support/slug.rs` |
| `MAX_*_LINKS` constants | `handlers/project.rs:568`, `handlers/game.rs:852` | `support/links.rs` |

`support/` is not a grab-bag: each file owns exactly one cross-cutting concern,
takes its dependencies as parameters (it does not reach into `AppState` for
feature-specific state), and is generic over the aggregate where the two copies
differ only in their error type.

`handlers/common.rs` (121 lines today) is dissolved into `support/cover.rs`;
there is no longer a `common.rs` inside `handlers/`.

## 3.7 Target allocation of oversized modules

Each table below states where the contents of one oversized file belong in the
target. These are **target assignments**, not migration steps.

### `handlers/v86.rs` — 3774 → 9 files

| Target file | Receives |
| --- | --- |
| `v86/mod.rs` | `routes()`, re-exports |
| `v86/constants.rs` | `MANIFEST_MAX_BYTES`, `V86_SAVE_*`, `V86_MEMORY_SIZE`, `V86_VGA_MEMORY_SIZE`, `V86_STATE_VERSION`, `V86_TOPOLOGY_VERSION`, `ZSTD_MAGIC`, `V86_SNAPSHOT_MAX_BYTES`, `SAVE_FILE_MAX_*` |
| `v86/dto.rs` | `V86SystemSpecs`, `StartSystemUpload*`, `GameDiskPlan`, `GameVariantPlan`, `GameBuildPlans`, `StartGameUpload*`, `DiskUploadSpec`, `VariantUploadSpec`, `StartUploadResponse`, `ChunkUploadResponse`, `StartSnapshotUploadRequest`, `SnapshotStatusResponse`, `UpdateSystemRequest`, `ActiveSystemsQuery`, `V86RuntimeDescriptor`, `VariantDescriptor`, `V86SystemResponse`, `V86SystemVersionResponse` |
| `v86/manifest.rs` | `validate_manifest`, `normalize_manifest_path`, `parse_manifest_fields`, `split_asset`, `parse_mouse_config`, `MouseConfig`, `key_index`, `parse_variants`, `VariantSpec`, `resolve_for`, `parse_system_specs`, `resolve_system_machine`, `v86_vga_memory_size_for`, `save_files_from_manifest`, `validate_save_file` |
| `v86/systems.rs` | `list_systems`, `list_public_systems`, `list_active_systems`, `start_system_upload`, `upload_system_part`, `abort_system_upload`, `complete_system_upload`, `update_system`, `get_system_upload_status`, `get_server_status`, `delete_system_version`, `delete_system`, `validate_memory_size_mb` |
| `v86/games.rs` | `start_game_upload`, `upload_game_disk_part`, `upload_game_variant_iso`, `complete_game_upload`, `attach_ready_game_tx`, `abort_game_upload`, `get_game_upload_status` |
| `v86/snapshots.rs` | `get_game_snapshot`, `start_snapshot_upload`, `append_snapshot_chunk`, `complete_snapshot_upload`, `abort_snapshot_upload`, `delete_game_snapshot` |
| `v86/runtime.rs` | `runtime_descriptor`, `runtime_descriptor_for`, `get_game_capture_runtime`, `get_game_launcher` |
| `v86/serving.rs` | `get_snapshot_blob`, `get_system_chunk`, `get_game_chunk`, `get_game_disk_chunk`, `get_game_iso` |
| `v86/saves.rs` | `get_game_save`, `put_game_save`, `delete_game_save` |
| `v86/tests.rs` | the existing 105-line inline test block |

### `handlers/game.rs` — 2234 → 6 files

| Target file | Receives |
| --- | --- |
| `game/mod.rs` | `routes()`, re-exports |
| `game/dto.rs` | `CheckQuery`, `CheckResponse`, `GameData`, `GamePatchData`, `StartJsDosUpload*`, `JsDosUploadResponse`, `CompleteJsDosUploadResponse`, `FileData`, `DeleteGameQuery`, `LatestGamesQuery`, `FeaturedGamesQuery`, `SetGameFeaturedBody`, `GameStats`, `GameCard`, `LatestGamesResponse`, `FeaturedGamesResponse` |
| `game/read.rs` | `check_game`, `get_game_by_slug`, `get_game_details`, `get_all_games`, `get_latest_games`, `get_featured_games` |
| `game/write.rs` | `new_game`, `update_game`, `publish_game`, `delete_game_draft`, `restore_game`, `purge_game_now`, `set_game_featured`, `repoint_game_system`, `change_cover` |
| `game/jsdos.rs` | `start_jsdos_upload`, `append_jsdos_chunk`, `complete_jsdos_upload`, `abort_jsdos_upload`, `get_jsdos_bundle`, `validate_jsdos_bundle`, `jsdos_temp_path`, `jsdos_storage_key`, `game_slug` |
| `game/response.rs` | `GameResponse`, `game_response`, `From<GameSnapshot> for GameCard`, `UpdateGameResponse` |
| `game/tests.rs` | the existing 133-line inline test block |

Everything else in this file (multipart parsing, inline media upload, zip
extraction, link and demo-URL validation, ownership guards) moves to
`web/api/support/` per §3.6.

### `handlers/project.rs` — 1560 → 6 files

| Target file | Receives |
| --- | --- |
| `project/mod.rs` | `routes()`, re-exports |
| `project/dto.rs` | `CheckQuery`, `CheckResponse`, `ProjectData`, `ProjectPatchData`, `StartJsDosUpload*`, `JsDosUploadResponse`, `CompleteJsDosUploadResponse`, `FileData`, `DeleteProjectQuery`, `LatestProjectsQuery`, `FeaturedProjectsQuery`, `SetProjectFeaturedBody`, `ProjectStats`, `ProjectCard`, `LatestProjectsResponse`, `FeaturedProjectsResponse` |
| `project/read.rs` | `check_project`, `get_project_by_slug`, `get_project_details`, `get_all_projects`, `get_latest_projects`, `get_featured_projects` |
| `project/write.rs` | `new_project`, `update_project`, `publish_project`, `delete_project_draft`, `restore_project`, `purge_project_now`, `set_project_featured`, `change_cover` |
| `project/demo.rs` | demo-ZIP and js-dos upload orchestration specific to projects |
| `project/response.rs` | `ProjectResponse`, `project_response`, `From<ProjectSnapshot> for ProjectCard`, `DelegatedGameResponse`, `UpdateProjectResponse` |

Shared helpers move to `support/` per §3.6.

### `handlers/post.rs` — 1527 → 7 files

| Target file | Receives |
| --- | --- |
| `post/mod.rs` | `routes()`, re-exports |
| `post/dto.rs` | `CheckQuery`, `CheckResponse`, `PostData`, `PostPatchData`, `FileData`, `SearchPostQuery`, `SearchTagsQuery`, `GetTagPostsQuery`, `GetFeaturedPostsBody`, `GetFeaturedPostsQuery`, `CommentsQuery`, `NewCommentBody` |
| `post/read.rs` | `check_post`, `get_post_by_slug`, `get_post_details`, `get_categories`, `search`, `search_tags`, `get_posts_by_tag`, `get_featured_posts`, `get_latest_posts`, `get_related_posts` |
| `post/write.rs` | `new_post`, `update_post`, `publish`, `set_related_posts`, `set_post_featured`, `change_cover` |
| `post/comments.rs` | `new_comment`, `get_comments` |
| `post/media.rs` | `push_view`, `push_like`, inline-media detection and bundling |
| `post/response.rs` | `PostResponse`, `PostSeriesResponse`, `CategoryResponse`, `GetCategoriesResponse`, `UpdatePostResponse`, `SearchPostResult`, `SearchTagResult`, `GetRelatedPostsResponse`, `Post`, `From<PostSnapshot> for Post` |

### `persistence/post.rs` — 1690 → 8 files

| Target file | Receives |
| --- | --- |
| `post/mod.rs` | `PostServiceImpl`, `new()`, the `impl PostService` block (thin, delegating) |
| `post/rows.rs` | `PostRow`, `PostContentRow`, `PostSearchRow`, `TagRow`, `TagSummaryRow`, `MediumUsageRow`, `MediumUsageWithNameRow`, `PostDetailsRow` |
| `post/mapping.rs` | `PostRow::into_snapshot`, `hydrate_post_rows`, row→entity conversion |
| `post/links.rs` | `resolve_tag_ids`, `link_post_tags`, `link_post_media`, `MENTION_RE`, `MAX_TAGS_PER_POST` |
| `post/read.rs` | `get_posts`, `check_slug`, `get_categories`, `search`, `search_tags`, `get_posts_by_tag`, `get_post`, `get_featured_post_snapshots`, `get_latest_post_snapshots`, `get_post_details`, `get_related_posts` |
| `post/write.rs` | `new_post`, `update_post`, `publish`, `set_related_posts`, `set_post_featured`, `update_post_cover` |
| `post/comments.rs` | `post_new_comment`, `post_new_anonymous_comment`, `get_comments`, `push_new_view`, `push_new_like` |

### `persistence/audiobook.rs` — 1406 → 8 files

| Target file | Receives |
| --- | --- |
| `audiobook/mod.rs` | `AudiobookServiceImpl`, `new()`, the `impl AudiobookService` block |
| `audiobook/rows.rs` | row structs |
| `audiobook/mapping.rs` | `attach_snapshot_tags`, snapshot assembly |
| `audiobook/read.rs` | `get_audiobooks`, `get_public_audiobooks`, `get_audiobook`, `get_public_audiobook`, `list_audiobook_tags`, `check_audiobook_slug` |
| `audiobook/write.rs` | `new_audiobook`, `update_audiobook`, `set_audiobook_cover`, `change_audiobook_status`, `delete_audiobook` |
| `audiobook/tracks.rs` | `resequence_tracks`, `load_tracks`, `add_track`, `update_track`, `remove_track`, `reorder_tracks` |
| `audiobook/tags.rs` | `replace_tags`, `load_tags` |
| `audiobook/validation.rs` | `MAX_*` constants, `is_cover_supported`, `is_audio_supported`, `assert_owned` |
| `audiobook/medium.rs` | `PreparedCover`, `store_medium` |

### `web/graphql/resolvers.rs` — 1137 → 11 files

`resolvers.rs` is one `#[Object] impl QueryRoot` block holding twenty fields
spanning every aggregate. async-graphql merges multiple `#[Object] impl QueryRoot`
blocks into one type, so the target splits it by domain:

| Target file | Receives |
| --- | --- |
| `graphql/schema.rs` | `QueryRoot`, `BlogSchema`, `build_schema` |
| `graphql/query/users.rs` | `users` |
| `graphql/query/posts.rs` | `posts` |
| `graphql/query/comments.rs` | `comments` |
| `graphql/query/media.rs` | `media` |
| `graphql/query/series.rs` | `series`, `series_posts` |
| `graphql/query/taxonomy.rs` | `tags`, `categories` |
| `graphql/query/stats.rs` | `db_stats`, `overview` |
| `graphql/query/dashboard.rs` | `dashboard_posts`, `dashboard_projects` |
| `graphql/query/featured.rs` | `featured_posts`, `featured_projects` |
| `graphql/query/detail.rs` | `post_detail`, `project_detail`, `related_posts` |
| `graphql/query/slug.rs` | `check_slug`, `check_project_slug` |

`types.rs` (267 lines, 22 types) splits into `graphql/types/<aggregate>.rs` and
`rows.rs` (180 lines, 15 rows) into `graphql/rows/<aggregate>.rs`.

### `web/server/mod.rs` — 778 → 11 files

| Target file | Receives |
| --- | --- |
| `server/mod.rs` | re-exports only |
| `server/state.rs` | `AppState`, `AppConfig`, `DatabaseSource`, `artifact_base_url()` |
| `server/config/database.rs` | `DatabaseSource::from_env` |
| `server/config/auth.rs` | `AuthConfig` construction, JWT settings |
| `server/config/media.rs` | `MediaConfig`, `from_env`, allowed-type lists |
| `server/config/project_demo.rs` | `ProjectDemoConfig`, `from_env`, all v86/js-dos limits |
| `server/config/storage.rs` | `ObjectStore` selection wiring |
| `server/config/mail.rs` | `MailConfig`, `MailTransportConfig` |
| `server/config/cors.rs` | allowed-origin resolution |
| `server/lifecycle.rs` | `HTTPServer`, `new/set_addr/set_port/set_db/start` |
| `server/maintenance/upload_sessions.rs` | `cleanup_orphaned_uploads` |
| `server/maintenance/trash.rs` | `purge_expired_trash` |
| `server/maintenance/game_artifacts.rs` | `cleanup_game_artifacts` |
| `server/maintenance/reading_times.rs` | `backfill_reading_times` |

### `web/api/router.rs` — 769 → composer + per-feature routers

`build_router` is currently a single 719-line function that also builds CORS,
defines the compression predicate, and inlines every route table.

| Target file | Receives |
| --- | --- |
| `api/router.rs` | `build_router` as a composer: nest each feature's `routes()` and apply global layers |
| `api/layers.rs` | CORS origin parsing and `CorsLayer`; the `CompressionLayer` predicate; `TraceLayer` policy |
| each `handlers/<feature>/mod.rs` | `pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>>` — the route table and its auth/body-limit layers move next to the handlers they guard |
| `handlers/mod.rs` | module declarations only, in alphabetical order (`common` disappears) |

Per-feature routers make each auth tier visible at the feature that owns it,
instead of requiring a reader to hold 719 lines of `Router::new().merge(...)` in
their head to answer "what protects `/posts/new`".

### Remaining files over the ceiling

| File | Lines | Target |
| --- | --- | --- |
| `persistence/media.rs` | 984 | `media/{mod,rows,hashing,aliases,crud,search,files}.rs` |
| `persistence/project.rs` | 870 | `project/{mod,rows,mapping,read,write,links}.rs` |
| `bin/sync-pull.rs` | 847 | `bin/sync-pull/{main,cli,plan,download,rewrite,tests}.rs` |
| `persistence/dashboard.rs` | 742 | `dashboard/{mod,rows,overview,posts,projects,users,tags,trash,analytics}.rs` |
| `persistence/game.rs` | 726 | `game/{mod,rows,mapping,read,write}.rs` |
| `persistence/series.rs` | 639 | `series/{mod,rows,read,write}.rs` |
| `handlers/audiobook.rs` | 629 | `audiobook/{mod,dto,read,write,tracks,response}.rs` |
| `persistence/auth.rs` | 624 | `auth/{mod,rows,tokens,sessions,credentials}.rs` |
| `handlers/media.rs` | 564 | `media/{mod,dto,serving,upload,manage,aliases}.rs` |
| `mail/mod.rs` | 534 | `mail/{mod,smtp,brevo,messages/*}.rs` |
| `persistence/newsletter.rs` | 525 | `newsletter/{mod,rows,subscribers,campaigns}.rs` |
| `storage/fs.rs` | 476 | `storage/fs.rs` + `storage/fs/tests.rs` (135 test lines move out) |
| `mail/templates.rs` | 473 | `mail/templates/{mod,layout,auth,newsletter}.rs` |
| `sync/mod.rs` | 469 | `sync/{mod,manifest,snapshot,files,rewrite}.rs` |
| `handlers/user.rs` | 402 | `user/{mod,dto,profile,avatar}.rs` |

### Baseline vs. target

| Metric | Today | Target |
| --- | --- | --- |
| Files in `src/` | 106 | ~215 |
| Total lines | ~31,400 | ~31,400 (unchanged) |
| Files over 400 lines | 20 (22,559 lines = 72%) | 0 |
| Files over 1000 lines | 7 (13,328 lines = 42%) | 0 |
| Largest production file | 3,774 | ≤ 400 |
| Largest `mod.rs` | 778 (`server/mod.rs`) | ≤ 200 |
| Duplicated helper definitions | 18 symbols across 3 modules | 0 |

## 3.8 Testing layout

| Test kind | Location | Rationale |
| --- | --- | --- |
| Unit tests for a module | `<module>/tests.rs`, declared `#[cfg(test)] mod tests;` | Production files stay inside budget; the test is discoverable from the file it covers. |
| Integration tests | `tests/*.rs` (unchanged) | Already correct: `audiobook_persistence.rs`, `post_persistence.rs`, `migration_test.rs`, `validation.rs`. |
| HTTP-level tests | `tests/http/` when introduced | Feature-scoped, mirroring `handlers/<feature>/`. |

All eleven inline `#[cfg(test)]` blocks in `src/` move to sibling `tests.rs`
files. Test modules are exempt from the file budget.

## 3.9 Naming and surface conventions

- **Directories** use `mod.rs` (matches the existing codebase; no mixed
  conventions).
- **Handler packages** use `mod.rs` for `routes()` plus fixed sub-file names:
  `dto.rs`, `read.rs`, `write.rs`, `response.rs`, and one file per distinct
  sub-capability (`comments.rs`, `tracks.rs`, `jsdos.rs`, `serving.rs`, …).
- **Persistence packages** use `mod.rs`, `rows.rs`, `mapping.rs`, `read.rs`,
  `write.rs` plus capability files.
- **`dto.rs` owns every wire type** for its feature: request bodies, query
  params, and response shapes. Handlers do not declare structs inline.
- **`response.rs` owns conversions** from domain entities to wire types
  (`From<X> for Y`, `x_response(x)`).
- **`lib.rs` keeps its current four public roots** (`application`, `domain`,
  `helper`, `infrastructure`). The internal split changes; the crate's public
  surface does not. This is what makes the refactor behaviour-preserving by
  construction.
- **`AppState` field names and service trait names do not change.** They are the
  contract between the HTTP layer and the adapter layer.

---

# Part B — Target Documentation Architecture

## 4.1 What is wrong with the documentation today

Eleven tracked markdown files, plus one untracked plan. The problems are
structural, not editorial.

| Problem | Evidence |
| --- | --- |
| **The root README describes a subsystem, not the repository.** | `README.md` is titled "Blog Platform" and states "this README focuses on the blog system under `blog/`", yet the repository also contains `portfolio/`, `ops/`, `scripts/`, `.github/`. There is no repository-level entry point. |
| **Configuration is documented four times, and the copies disagree.** | Backend env vars appear in `README.md` (30 rows), `blog/README.md` (12 rows), `blog/backend/readme.md` (12 rows), and `blog/frontend/README.md` (5 rows). `README.md` documents `STORAGE_BACKEND`, `R2_*`, and the `PROJECT_V86_*` family; `blog/backend/readme.md` documents none of them. |
| **A documentation link is broken by filename case.** | `blog/README.md:70` links to `backend/README.md`; the file is `blog/backend/readme.md`. This resolves on case-insensitive macOS and 404s on case-sensitive Linux and on GitHub. |
| **Two documentation links point at files that do not exist.** | `.github/README.md` references `journal-logs/2026-04-09-flyio-cloudflare-migration.md` and `journal-logs/2026-04-10-oracle-cloud-migration.md`. There is no `journal-logs/` directory anywhere in the repository. |
| **The API reference is incomplete.** | `blog/backend/readme.md` documents auth, users, posts, series, audiobooks, media, dashboard, mail, and GraphQL. `router.rs` additionally exposes `/games`, `/v86`, `/sync`, `/newsletter`, `/analytics`, `/dashboard/trash`, `/dashboard/projects`, `/dashboard/tags`, `/posts/id/:id/restore`, `/posts/id/:id/purge`, and `/users/:username/comments` — none of which appear. |
| **One "README" is not a README.** | `ops/nginx/README.md` is twelve lines of `#` comment prose with no title, no headings, and no structure. |
| **Proposal documents have no index and no lifecycle.** | `blog/docs/` holds three design documents. Two are tracked, one (`v86-registry-saves-plan.md`) is untracked. There is no index, and status is stated inconsistently in prose ("Status: proposal", "Status: **implemented** (2026-09-11)"). |
| **No contributor documentation exists.** | No `CONTRIBUTING.md`. Commit conventions, formatting, lint gates, and migration policy are undocumented, though `Makefile`, `rustfmt`, `clippy -D warnings`, and Prettier all enforce them. |
| **The architecture has no single home.** | The layer model is described in three sentences in `blog/backend/readme.md`, the request flow in `README.md` and `blog/README.md` (differently), and deployment across `README.md`, `blog/README.md`, and `.github/README.md`. |

## 4.2 Target documentation tree

```text
/
├── README.md                        # repository hub — see §4.3
├── CONTRIBUTING.md                  # conventions: commits, style, lint, docs rules
│
├── docs/                            # the documentation proper
│   ├── README.md                    # navigation index for all of docs/
│   │
│   ├── architecture/
│   │   ├── overview.md              # system context, components, trust boundaries
│   │   ├── backend.md               # layer model, module map, dependency rules, budgets
│   │   ├── frontend.md              # SvelteKit structure, the three request paths
│   │   ├── data-model.md            # SQLite schema, aggregates, migration policy
│   │   └── deployment.md            # VM, nginx, Docker, GHCR, DNS/TLS topology
│   │
│   ├── guides/
│   │   ├── setup.md                 # prerequisites, full-stack and standalone paths
│   │   ├── configuration.md         # every env var, per service, single source of truth
│   │   ├── development.md           # make targets, migrations, lint/fmt, tests
│   │   ├── deployment.md            # CI/CD pipeline and the manual emergency path
│   │   ├── operations.md            # backups, sync-pull, R2↔fs, restore
│   │   └── troubleshooting.md       # known failure modes and their symptoms
│   │
│   ├── reference/
│   │   ├── api-rest.md              # complete REST surface, derived from router.rs
│   │   ├── api-graphql.md           # admin GraphQL surface
│   │   ├── media-and-storage.md     # ObjectStore, key layout, range serving
│   │   ├── auth-and-roles.md        # token lifecycle, role tiers, guards
│   │   └── error-model.md           # per-aggregate error enums and HTTP mapping
│   │
│   └── decisions/
│       ├── README.md                # ADR index
│       └── 000N-<slug>.md           # one file per architectural decision
│
├── blog/
│   ├── README.md                    # blog subsystem hub — trimmed, links into docs/
│   ├── backend/
│   │   └── README.md                # RENAMED from readme.md — backend hub
│   ├── frontend/
│   │   └── README.md                # frontend hub — trimmed
│   └── docs/
│       ├── README.md                # NEW — index of design proposals
│       ├── draft-body-collapse.md
│       ├── editor-feedback-ux.md
│       └── v86-registry-saves-plan.md
│
├── ops/nginx/README.md              # rewritten as a real README
├── portfolio/README.md
├── scripts/emergency/README.md
└── .github/README.md                # CI/CD and infrastructure
```

## 4.3 The README contract

Every README in the repository answers exactly five questions, in this order, and
nothing more:

1. **What is this?** — one paragraph, no more.
2. **How do I run it?** — the shortest path to a working instance.
3. **How do I configure it?** — a short table that links to
   `docs/guides/configuration.md` rather than restating it.
4. **Where does the code live?** — the directory map for this scope.
5. **Where do I go deeper?** — links into `docs/`.

| README | Audience | Ceiling |
| --- | --- | --- |
| `/README.md` | anyone arriving at the repository | 150 lines |
| `blog/README.md` | someone running the blog stack | 100 lines |
| `blog/backend/README.md` | someone working on the backend | 150 lines |
| `blog/frontend/README.md` | someone working on the frontend | 100 lines |
| `portfolio/README.md` | someone working on the portfolio site | 80 lines |
| `ops/nginx/README.md` | someone operating the VM's nginx | 60 lines |
| `.github/README.md` | someone changing CI/CD | 100 lines |
| `scripts/emergency/README.md` | someone deploying manually | 80 lines |

The rule that makes this work: **a README may summarise, but it may not be the
only place a fact lives.** Every table longer than ~15 rows belongs in `docs/`.

## 4.4 Topic ownership — single source of truth

| Topic | Canonical home | READMEs link to it |
| --- | --- | --- |
| Environment variables (all services) | `docs/guides/configuration.md` | root, blog, backend, frontend |
| REST route reference | `docs/reference/api-rest.md` | backend, root |
| GraphQL reference | `docs/reference/api-graphql.md` | backend |
| Request flow / proxying | `docs/architecture/overview.md` | root, blog, frontend |
| Backend layer model and module map | `docs/architecture/backend.md` | backend, root |
| Database schema and migrations | `docs/architecture/data-model.md` | backend |
| Deployment topology (nginx, TLS, DNS, VM) | `docs/architecture/deployment.md` | root, blog, `.github` |
| CI/CD pipeline and secrets | `.github/README.md` + `docs/guides/deployment.md` | root |
| Backup, restore, sync-pull, R2↔fs | `docs/guides/operations.md` | backend, root |
| Media and object storage | `docs/reference/media-and-storage.md` | backend |
| Auth and role tiers | `docs/reference/auth-and-roles.md` | backend |
| Feature design proposals | `blog/docs/` | blog, backend |

This table is the answer to the current four-way env-var drift: the fact has one
home and three links.

## 4.5 `blog/docs/` becomes a governed proposal archive

`blog/docs/` keeps its purpose — feature-level design documents — and gains
rules:

- **A `README.md` index** listing every document with its status, scope, and date.
- **Status front-matter** on every document, from a fixed vocabulary:
  `proposal`, `accepted`, `implemented`, `superseded`, `rejected`.
  Today the status is prose inside the body and inconsistent between files.
- **A declared scope line** naming the directories the document touches. Two of
  the three current documents already do this informally.
- **Superseded documents are kept, not deleted,** with a link to their
  replacement. They are the record of why the current design looks the way it
  does.
- **`v86-registry-saves-plan.md` is tracked** like the other two.

Documents in `blog/docs/` are engineering artefacts, not user documentation. They
do not appear in `docs/README.md`; the index links to `blog/docs/README.md`.

## 4.6 Documentation conventions

Applies to every markdown file except `blog/frontend/static/zstd/NOTICE.md`, which
is a verbatim third-party notice and must not be edited.

| Rule | Detail |
| --- | --- |
| One H1 per file | The filename and the H1 agree. |
| Heading hierarchy | No skipped levels. |
| Relative links only | No absolute local paths, no `file://`, no machine-specific paths. |
| Link targets must exist | Renames update every referrer in the same change — this is what currently breaks `blog/README.md`. |
| Fenced code blocks carry a language | ```bash, ```text, ```sql, ```rust, ```nginx, ```json. |
| Diagrams | ASCII inside ```text fences. Consistent with the existing READMEs. |
| Tables | Used for enumerable facts (env vars, routes, limits). Prose for reasoning. |
| Prose voice | Present tense, second person for instructions, no marketing language. |
| No duplicated facts | If a fact appears twice, one occurrence becomes a link. |
| Dates | ISO 8601 (`2026-09-14`) — matches migration filenames. |
| Language | English. |

## 4.7 Current → target documentation map

| Current file | Lines | Disposition | Target |
| --- | --- | --- | --- |
| `README.md` | 431 | Split and trim | Hub → `/README.md`; architecture → `docs/architecture/overview.md`; stack table → `docs/architecture/overview.md`; setup → `docs/guides/setup.md`; Makefile table → `docs/guides/development.md`; env tables → `docs/guides/configuration.md`; deployment → `docs/guides/deployment.md`; production expectations and limitations → `docs/guides/operations.md`; feature inventory → `docs/architecture/overview.md` |
| `blog/README.md` | 70 | Trim | Hub; request-flow diagram → `docs/architecture/overview.md`; env tables → `docs/guides/configuration.md`; sub-project links updated |
| `blog/backend/readme.md` | 301 | **Rename** + split | `blog/backend/README.md` hub; layer model → `docs/architecture/backend.md`; env table → `docs/guides/configuration.md`; API routes → `docs/reference/api-rest.md`; ObjectStore / R2↔fs → `docs/reference/media-and-storage.md` + `docs/guides/operations.md`; sync keys → `docs/guides/operations.md`; backup → `docs/guides/operations.md` |
| `blog/frontend/README.md` | 67 | Trim | Hub; request-flow diagram → `docs/architecture/frontend.md`; env table → `docs/guides/configuration.md` |
| `.github/README.md` | 100 | Keep + fix | CI/CD reference; **remove the two dead `journal-logs/` links**; nginx snippets → `docs/architecture/deployment.md`; secrets table stays |
| `ops/nginx/README.md` | 12 | **Rewrite** | Real README: what is managed here vs. on the VM, the three conf files, the certbot one-time step |
| `scripts/emergency/README.md` | 45 | Keep | Linked from `docs/guides/deployment.md` as the manual deploy path |
| `portfolio/README.md` | 58 | Keep + expand | Standalone project hub |
| `blog/docs/draft-body-collapse.md` | 279 | Keep | Status front-matter; indexed in `blog/docs/README.md` |
| `blog/docs/editor-feedback-ux.md` | 269 | Keep | Status front-matter; indexed |
| `blog/docs/v86-registry-saves-plan.md` | 250 | **Track** | Add to git; status front-matter; indexed |
| `blog/frontend/static/zstd/NOTICE.md` | 56 | Keep verbatim | Exempt from conventions |
| `blog/frontend/build/…/NOTICE.md` | 56 | Ignore | Build output |
| `blog/frontend/.svelte-kit/…/NOTICE.md` | 56 | Ignore | Build output |
| `.zcode/plans/*.md` | 137 | Ignore | `.zcode/` is gitignored; session plans are not repository documentation |
| `.workbuddy-ai/memory/*.md` | 101 | Ignore | Agent workspace state |

New files to be created:

| Target file | Purpose |
| --- | --- |
| `/CONTRIBUTING.md` | Commit conventions (the repo already uses conventional commits with scopes), formatting and lint gates, migration policy, PR flow, and the documentation rules in §4.6 |
| `/docs/README.md` | Documentation index; the entry point that makes the tree navigable |
| `docs/architecture/overview.md` | System context and the three frontend request paths |
| `docs/architecture/backend.md` | The target architecture in Part A, as the enduring reference |
| `docs/architecture/frontend.md` | SvelteKit structure and SSR/proxy/media routing |
| `docs/architecture/data-model.md` | Aggregates, tables, migration policy |
| `docs/architecture/deployment.md` | nginx, TLS, DNS, VM, container topology |
| `docs/guides/setup.md` | Prerequisites and both local-run paths |
| `docs/guides/configuration.md` | Every env var, single source of truth |
| `docs/guides/development.md` | Make targets, migrations, lint, tests |
| `docs/guides/deployment.md` | CI/CD and manual deploy |
| `docs/guides/operations.md` | Backups, sync-pull, R2↔fs migration, restore |
| `docs/guides/troubleshooting.md` | Known failure modes |
| `docs/reference/api-rest.md` | Complete REST surface derived from `router.rs` |
| `docs/reference/api-graphql.md` | GraphQL surface |
| `docs/reference/media-and-storage.md` | ObjectStore and key layout |
| `docs/reference/auth-and-roles.md` | Token lifecycle and role tiers |
| `docs/reference/error-model.md` | Error enums and HTTP mapping |
| `docs/decisions/README.md` | ADR index |
| `blog/docs/README.md` | Proposal index |

## 4.8 Documentation governance

- **A doc and the code it describes change together.** The API reference is the
  sharpest case: `router.rs` is the source of truth, and a new route without a
  reference entry is an incomplete change.
- **Each `docs/` file declares its audience and its update trigger** in a short
  header, so a future reader knows when it goes stale.
- **The reference layer is derived, not authored.** `docs/reference/api-rest.md`
  should be reconstructable from `router.rs` route tables; the doc exists to add
  the prose that the router cannot express (auth tiers, body limits, semantics).
- **One fact, one home.** A fact in two files is a defect, per §4.4.
- **Link integrity is a review criterion.** The two broken references found today
  (§4.1) are exactly the class of defect this rule prevents.

---

## 5. Acceptance criteria for the target state

**Backend architecture**

- [ ] No production file in `src/` exceeds 400 lines; no `mod.rs` exceeds 200.
- [ ] `web/api/router.rs` contains no route table and no layer construction.
- [ ] No handler module imports another handler module.
- [ ] No handler module imports `infrastructure::persistence` directly.
- [ ] No symbol listed in §3.6 exists in more than one place.
- [ ] Every aggregate is addressable at the same relative position in all three
      layers.
- [ ] `domain` compiles with `sqlx` and `axum` removed from its dependency set.
- [ ] `lib.rs` exposes the same four public roots; `AppState` field names and
      service trait names are unchanged.
- [ ] No inline `#[cfg(test)] mod tests` block remains in a production file.
- [ ] The GraphQL boundary is either documented as a named exception or removed,
      per the decision in §3.4.

**Documentation architecture**

- [ ] `/README.md` describes the repository, not one subsystem, and is ≤ 150 lines.
- [ ] `/CONTRIBUTING.md` exists.
- [ ] `docs/README.md` links to every document in `docs/`, and every link resolves.
- [ ] Environment variables are documented exactly once, in
      `docs/guides/configuration.md`.
- [ ] The REST reference covers every route in `router.rs`.
- [ ] `blog/backend/readme.md` is renamed to `README.md`, and every referrer is updated.
- [ ] The dead `journal-logs/` links are gone from `.github/README.md`.
- [ ] `ops/nginx/README.md` is a structured README.
- [ ] `blog/docs/README.md` indexes all three proposals with declared statuses.
- [ ] `v86-registry-saves-plan.md` is tracked.
- [ ] Every markdown link in the repository resolves on a case-sensitive filesystem.
- [ ] No README exceeds its ceiling in §4.3.

---

## 6. Appendix — measured baseline

Captured from the working tree at the time of writing.

**Backend scale**

| Metric | Value |
| --- | --- |
| Rust files in `blog/backend/src/` | 106 |
| Total lines | 31,444 |
| Mean lines per file | ~297 |
| Files > 1000 lines | 7 — 13,328 lines (42.4%) |
| Files > 700 lines | 14 — 19,044 lines (60.6%) |
| Files > 400 lines | 20 — 22,559 lines (71.7%) |
| SQL migrations | 54 |
| Integration test files | 4 |
| Inline `#[cfg(test)]` blocks in `src/` | 11 |

**Largest production files**

| Lines | File |
| --- | --- |
| 3774 | `infrastructure/web/api/handlers/v86.rs` |
| 2234 | `infrastructure/web/api/handlers/game.rs` |
| 1690 | `infrastructure/persistence/post.rs` |
| 1560 | `infrastructure/web/api/handlers/project.rs` |
| 1527 | `infrastructure/web/api/handlers/post.rs` |
| 1406 | `infrastructure/persistence/audiobook.rs` |
| 1137 | `infrastructure/web/graphql/resolvers.rs` |
| 984 | `infrastructure/persistence/media.rs` |
| 870 | `infrastructure/persistence/project.rs` |
| 847 | `bin/sync-pull.rs` |
| 778 | `infrastructure/web/server/mod.rs` |
| 769 | `infrastructure/web/api/router.rs` |
| 742 | `infrastructure/persistence/dashboard.rs` |
| 726 | `infrastructure/persistence/game.rs` |

**Documentation inventory**

| Lines | File | Tracked |
| --- | --- | --- |
| 431 | `README.md` | yes |
| 301 | `blog/backend/readme.md` | yes |
| 279 | `blog/docs/draft-body-collapse.md` | yes |
| 269 | `blog/docs/editor-feedback-ux.md` | yes |
| 250 | `blog/docs/v86-registry-saves-plan.md` | **no** |
| 100 | `.github/README.md` | yes |
| 70 | `blog/README.md` | yes |
| 67 | `blog/frontend/README.md` | yes |
| 58 | `portfolio/README.md` | yes |
| 56 | `blog/frontend/static/zstd/NOTICE.md` | yes |
| 45 | `scripts/emergency/README.md` | yes |
| 12 | `ops/nginx/README.md` | yes |

**Defects found while measuring**

| Defect | Location |
| --- | --- |
| Link target filename case mismatch (`backend/README.md` vs `readme.md`) | `blog/README.md:70` |
| Two links to a non-existent `journal-logs/` directory | `.github/README.md` |
| `/games`, `/v86`, `/sync`, `/newsletter`, `/analytics` and several `/dashboard` and `/posts` routes absent from the API reference | `blog/backend/readme.md` |
| Backend env vars documented in four places with disagreeing content | root, `blog/`, `blog/backend/`, `blog/frontend/` |
| `README.md` is a subsystem README presented as the repository README | `/README.md:1-5` |
| Proposal document untracked while its siblings are tracked | `blog/docs/v86-registry-saves-plan.md` |
| "README" with no title, headings, or structure | `ops/nginx/README.md` |

---

*This plan is target-state only. Once the documentation tree in Part B exists,
this document should be re-homed as `docs/decisions/000N-backend-modularization.md`
so that the rationale sits beside the architecture it defines.*
