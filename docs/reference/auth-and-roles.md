# Auth and Roles

Audience: backend contributors and dashboard users. Update trigger: a change
to the token lifecycle or role tiers.

## Tokens

- Access and refresh tokens are JWTs (HS256) signed with `JWT_SECRET`.
- Lifetimes: `ACCESS_JWT_EXP_HOURS` (24), `REFRESH_JWT_EXP_HOURS` (360).
- `POST /auth/login` issues both; `POST /auth/refresh` exchanges a refresh
  token for a new pair.

## Route guard tiers

| Tier | Middleware | Applies to |
| --- | --- | --- |
| public | — | Reader endpoints |
| optional | `optional_user_guard` | Slugs, user posts/comments, saves |
| protected | `user_guard` | `/users/me/*`, profile edits |
| mod | `mod_check` (+ `user_guard`) | Authoring, media, dashboard overview |
| admin | `admin_check` (+ `user_guard`) | Purges, featured picks, sync keys, v86 systems |
| sync | `sync_key_guard` | `/sync/*` (pull-only keys) |
| author-key | `author_key_guard` | `/author/*` (one book per key; opt-in) |

Guard composition per route group is visible in each feature's `routes()`
function (`src/infrastructure/web/api/handlers/<feature>/`).

## Roles

Roles live in the database (`users.role`): `user`, `moderator`, `admin`.
Initial admin setup is manual (database seed). Dashboard role management is
admin-only.

## Sync keys

Sync keys (`bsk_…`) are admin-issued, shown once, stored as SHA-256 hashes,
TTL-bounded, and revocable from the dashboard. They authorize the pull-only
`/sync` API — see [../guides/operations.md](../guides/operations.md).

## Author keys

Author keys (`abk_…`) are the opposite shape: they reach **one audiobook**, not
the whole database, and they can write. They exist so an author can drive their
own catalogue from a script — see
[../reference/api-rest.md](../reference/api-rest.md).

- **Issued by** the book's creator only (`audiobooks.user_id`), from that
  book's dashboard page, with a normal session token. There is no
  admin-issued-on-behalf-of path.
- **Scoped to the book.** The key row carries `audiobook_id`; the guard
  compares it with the id in the URL and answers 404 on a mismatch.
- **Never widens the session.** `role` and the book's owner are re-read on every
  request, so a demotion or a reassignment takes effect immediately. A
  moderator/admin account is still required.
- **Expiry is optional.** `expires_at` is nullable and `NULL` means "until
  revoked"; issuance can pass `ttl_hours`.
- **Revocable per book**, and deleted with the book (both foreign keys cascade).
- **Off by default, per book.** `audiobooks.author_api_enabled` (default `0`) is
  switched on by the book's creator from that book's dashboard page. There is
  no deployment-wide switch: one book being on says nothing about any other,
  and a book that is off answers 404 even to a valid key. Turning it off pauses
  the book's keys rather than deleting them.
- `scopes` is `read` or `read,write`. A read-only key can read the manifest and
  download chapters but cannot change anything.

The whole feature is documented as a proposal-turned-implementation in
[`blog/docs/audiobook-author-api.md`](../../blog/docs/audiobook-author-api.md).
