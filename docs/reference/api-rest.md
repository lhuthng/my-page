# REST API Reference

Audience: API consumers and backend contributors. Update trigger: any route
change — `router.rs` and the per-feature `routes()` tables are the source of
truth, and a route without an entry here is an incomplete change.

Base URL: the backend serves everything directly; in production the frontend
`/api/*` proxy also forwards to it.

Auth tiers:

- **public** — no token
- **optional** — token parsed if present (personalizes the response)
- **protected** — any logged-in user
- **mod** — moderator or admin
- **admin** — admin only
- **sync** — valid sync key (`bsk_…` bearer)

## Auth — `/auth` (public)

| Method | Path | Description |
| --- | --- | --- |
| POST | `/auth/login` | Issue access + refresh tokens |
| POST | `/auth/register` | Create a new account |
| GET | `/auth/verify-email` | Verify an email address |
| POST | `/auth/forgot-password` | Request a password reset |
| POST | `/auth/reset-password` | Reset the password |
| POST | `/auth/resend-verification` | Resend the verification email |
| POST | `/auth/refresh` | Exchange a refresh token |

## Users — `/users`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/users/{username}` | public | User profile |
| GET | `/users/` | public | Search users |
| GET | `/users/{username}/posts` | optional | Posts by user |
| GET | `/users/{username}/comments` | optional | Latest comments by user |
| GET | `/users/me` | protected | Own profile |
| PATCH | `/users/me/details` | protected | Update own details |
| PATCH | `/users/me/avatar` | protected | Upload avatar (20 MB) |
| GET | `/users/me/check-mod` | protected | Moderator check |

## Posts — `/posts`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/posts/s/{slug}` | optional | Post by slug |
| PUT | `/posts/id/{id}/comments/new` | optional | Add a comment (2 MB) |
| POST | `/posts/new` | mod | Create a draft |
| POST | `/posts/id/{id}` | mod | Publish |
| GET | `/posts/id/{id}` | mod | Details incl. unpublished |
| PATCH | `/posts/id/{id}` | mod | Update content/metadata |
| DELETE | `/posts/id/{id}` | mod | Soft-delete to trash |
| POST | `/posts/id/{id}/restore` | mod | Restore from trash |
| PATCH | `/posts/id/{id}/cover` | mod | Replace cover (100 MB) |
| PATCH | `/posts/id/{id}/related` | mod | Set related posts |
| DELETE | `/posts/id/{id}/purge` | admin | Hard purge |
| PUT | `/posts/id/{id}/featured` | admin | Toggle featured |
| GET | `/posts/id/{id}/comments` | public | Comments for a post |
| POST | `/posts/id/{id}/view` | public | Record a view |
| POST | `/posts/id/{id}/like` | public | Like a post |
| GET | `/posts/id/{id}/related` | public | Related posts |
| GET | `/posts/featured` | public | Featured posts |
| GET | `/posts/latest` | public | Latest posts |
| GET | `/posts/check` | public | Slug availability |
| GET | `/posts/categories` | public | List categories |
| GET | `/posts/` | public | Search posts |

## Tags — `/tags`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/tags/` | public | Search tags |
| GET | `/tags/{tag_slug}` | public | Posts for a tag |

## Projects — `/projects`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/projects/s/{slug}` | optional | Project by slug |
| POST | `/projects/new` | mod | Create a draft |
| GET | `/projects/all` | mod | Admin listing |
| POST | `/projects/id/{id}` | mod | Publish |
| GET | `/projects/id/{id}` | mod | Details incl. unpublished |
| PATCH | `/projects/id/{id}` | mod | Update |
| DELETE | `/projects/id/{id}` | mod | Soft-delete |
| POST | `/projects/id/{id}/restore` | mod | Restore |
| PATCH | `/projects/id/{id}/cover` | mod | Replace cover |
| DELETE | `/projects/id/{id}/purge` | admin | Hard purge |
| PUT | `/projects/id/{id}/featured` | admin | Toggle featured |
| GET | `/projects/latest` | public | Latest projects |
| GET | `/projects/featured` | public | Featured projects |
| GET | `/projects/check` | public | Slug availability |

## Games — `/games`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/games/s/{slug}` | optional | Game by slug (with v86 runtime descriptor when applicable) |
| GET/PUT/DELETE | `/games/s/{slug}/v86/saves` | optional | Per-user cloud save (floppy image) |
| POST | `/games/new` | mod | Create a draft |
| POST | `/games/id/{id}/jsdos/upload` | mod | Start js-dos bundle upload |
| PUT | `/games/id/{id}/jsdos/upload/{upload_id}/chunk/{index}` | mod | Append a chunk |
| POST | `/games/id/{id}/jsdos/upload/{upload_id}/complete` | mod | Finalize the bundle |
| DELETE | `/games/id/{id}/jsdos/upload/{upload_id}` | mod | Abort the upload |
| GET | `/games/all` | mod | Admin listing |
| POST | `/games/id/{id}` | mod | Publish |
| GET | `/games/id/{id}` | mod | Details |
| PATCH | `/games/id/{id}` | mod | Update |
| DELETE | `/games/id/{id}` | mod | Soft-delete |
| POST | `/games/id/{id}/restore` | mod | Restore |
| PATCH | `/games/id/{id}/cover` | mod | Replace cover |
| PUT | `/games/id/{id}/featured` | admin | Toggle featured |
| DELETE | `/games/id/{id}/purge` | admin | Hard purge |
| GET | `/games/s/{slug}/jsdos` | public | Serve the js-dos bundle |
| GET | `/games/s/{slug}/v86/{sha}/full.iso` | public | Serve a launcher CD |
| GET | `/games/s/{slug}/v86/{sha}/{part}` | public | Serve an ISO chunk |
| GET | `/games/s/{slug}/v86/disk/{sha}/{part}` | public | Serve a disk chunk |
| GET | `/games/latest` | public | Latest games |
| GET | `/games/featured` | public | Featured games |
| GET | `/games/check` | public | Slug availability |

## v86 — `/v86`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/v86/systems/active` | mod | Active systems |
| GET | `/v86/launcher` | mod | In-guest launcher file |
| POST | `/v86/games/upload` | mod | Start a game package upload |
| PUT | `/v86/games/upload/{id}/disk/{part}` | mod | Upload a disk part |
| PUT | `/v86/games/upload/{id}/iso/{variant}` | mod | Upload a launcher CD |
| POST | `/v86/games/upload/{id}/complete` | mod | Finalize |
| GET/DELETE | `/v86/games/upload/{id}` | mod | Status / abort |
| POST | `/v86/snapshots/upload` | mod | Start a snapshot upload |
| PUT | `/v86/snapshots/upload/{id}/chunk/{index}` | mod | Append a chunk |
| POST | `/v86/snapshots/upload/{id}/complete` | mod | Finalize |
| DELETE | `/v86/snapshots/upload/{id}` | mod | Abort |
| GET | `/v86/games/id/{id}/snapshot` | mod | Snapshot status list |
| DELETE | `/v86/games/id/{id}/snapshot/{variant}` | mod | Delete a snapshot |
| GET | `/v86/games/id/{id}/capture-runtime` | mod | Capture runtime (drafts allowed) |
| GET | `/v86/systems` | admin | List systems |
| GET | `/v86/systems/status` | admin | Upload server status |
| POST | `/v86/systems/upload` | admin | Start base-image upload |
| PUT | `/v86/systems/upload/{id}/part/{part}` | admin | Upload a part |
| POST | `/v86/systems/upload/{id}/complete` | admin | Finalize |
| GET/DELETE | `/v86/systems/upload/{id}` | admin | Status / abort |
| PATCH | `/v86/systems/{id}` | admin | Update system settings |
| DELETE | `/v86/systems/{id}` | admin | Delete a system |
| DELETE | `/v86/systems/{id}/versions/{version_id}` | admin | Delete a version |
| GET | `/v86/systems/public` | public | Published system versions |
| GET | `/v86/assets/systems/{sha}/{part}` | public | Serve a base-image chunk |
| GET | `/v86/snapshots/{sha}/state.zst` | public | Serve a snapshot blob |

## Series — `/series`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/series/public/all` | public | Published series |
| GET | `/series/all` | mod | All series incl. drafts |
| POST | `/series/new` | mod | Create a series |
| PATCH | `/series/id/{id}` | mod | Add a post |
| DELETE | `/series/id/{id}` | mod | Remove a post |
| GET | `/series/id/{id}/posts` | mod | Posts in a series |

## Audiobooks — `/audiobooks`

Tracks reference `media` rows; audio is served by `/media/i/{short_name}`
with HTTP Range support.

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/audiobooks/all` | mod | Admin catalogue (`?term=`, `?limit=`, `?offset=`) |
| GET | `/audiobooks/tags` | mod | Audiobook tag vocabulary with counts |
| POST | `/audiobooks/new` | mod | Create (multipart) |
| GET | `/audiobooks/id/{id}` | mod | Details with ordered tracks |
| PATCH | `/audiobooks/id/{id}` | mod | Update metadata (`translator: null` clears) |
| DELETE | `/audiobooks/id/{id}` | mod | Delete (tracks cascade, media kept) |
| PATCH | `/audiobooks/id/{id}/cover` | mod | Replace cover |
| POST | `/audiobooks/id/{id}/status` | mod | `draft` / `published` / `archived` |
| POST | `/audiobooks/id/{id}/tracks` | mod | Upload a track (100 MB) |
| PUT | `/audiobooks/id/{id}/tracks/order` | mod | Reorder (every id exactly once) |
| PATCH | `/audiobooks/id/{id}/tracks/{track_id}` | mod | Rename/re-duration/move |
| DELETE | `/audiobooks/id/{id}/tracks/{track_id}` | mod | Remove and close the gap |
| GET | `/audiobooks/check` | public | Slug availability |
| GET | `/audiobooks/public/all` | public | Published catalogue |
| GET | `/audiobooks/public/s/{slug}` | public | Published book with tracks |

## Media — `/media`

Static files are also served from `MEDIA_PATH` via `ServeDir` as a fallback.

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| POST | `/media/upload` | mod | Upload (100 MB) |
| GET | `/media/d/{name}` | mod | Details |
| PATCH | `/media/d/{name}` | mod | Update details |
| GET | `/media/d/{name}/aliases` | mod | List aliases |
| POST | `/media/d/{name}/aliases` | mod | Add an alias |
| PATCH | `/media/d/{name}/aliases/{alias}` | mod | Update an alias |
| DELETE | `/media/d/{name}/aliases/{alias}` | mod | Delete an alias |
| GET | `/media/s/{name}` | public | Shareable link |
| GET | `/media/i/{name}` | public | Serve a file (HTTP Range) |
| GET | `/media/all` | public | Search |

## Dashboard — `/dashboard` (mod) / admin sub-scope

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| GET | `/dashboard/overview` | mod | Summary stats |
| GET | `/dashboard/posts` | mod | Post management data |
| GET | `/dashboard/projects` | mod | Project management data |
| GET | `/dashboard/trash` | mod | Trash contents |
| GET | `/dashboard/users` | mod | User management data |
| GET | `/dashboard/newsletter/subscribers` | mod | Subscribers |
| GET | `/dashboard/newsletter/campaigns` | mod | Campaigns |
| POST | `/dashboard/newsletter/send` | mod | Send a campaign |
| GET | `/dashboard/analytics/countries` | admin | Visitor countries |
| PATCH | `/dashboard/tags/{tag_id}` | admin | Update a tag |
| DELETE | `/dashboard/tags/{tag_id}` | admin | Delete a tag |
| GET/POST | `/dashboard/sync-keys` | admin | List / create sync keys |
| DELETE | `/dashboard/sync-keys/{key_id}` | admin | Revoke a sync key |

## Sync — `/sync` (sync key)

| Method | Path | Description |
| --- | --- | --- |
| GET | `/sync/manifest` | Transfer manifest |
| GET | `/sync/database` | Consistent SQLite snapshot |
| GET | `/sync/media/{hash}` | A media file |
| GET | `/sync/demo/{kind}/{id}/{path}` | A demo file |
| GET | `/sync/artifact/{key}` | A v86/js-dos artifact |

## Analytics — `/analytics` (public)

| Method | Path | Description |
| --- | --- | --- |
| POST | `/analytics/visit` | Visitor beacon |

## Mail — `/mail`

| Method | Path | Auth | Description |
| --- | --- | --- | --- |
| POST | `/mail/contact-form` | CORS-restricted | Contact form delivery (`ALLOWED_ORIGIN`) |
| GET | `/mail/preview` | debug builds only | Template preview |

## Health

| Method | Path | Description |
| --- | --- | --- |
| GET | `/health` | Liveness probe (no DB) |

## GraphQL — `/graphql` (mod+)

`GET /graphql` serves the playground, `POST /graphql` executes queries.
See [api-graphql.md](api-graphql.md).
