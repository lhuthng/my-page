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
