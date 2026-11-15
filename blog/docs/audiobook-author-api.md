# Audiobook author API: hash-addressed uploads behind a secret key

Status: implemented 2026-11-15
Scope: `blog/backend` + `blog/frontend` (+ `docs/reference/api-rest.md`, `docs/reference/auth-and-roles.md`)

## Problem

Chapters reach the server one way today: the dashboard editor, one multipart
form per chapter, driven by a human in a browser
(`POST /audiobooks/id/{id}/tracks`).

That is fine for a 12-chapter book. It is not fine for the books this collection
is actually built around. A 900-chapter book re-encoded at a different bitrate
means 900 manual uploads, and the server has no way to tell an author's tooling
"this one is already here, skip it" — so every byte crosses the wire again even
when the audio is byte-identical to what is already stored.

The ask: let an author drive **their book** from a script, with a secret key,
and let hashes do the bookkeeping so nothing is uploaded twice.

## Decisions

| Question | Decision |
| --- | --- |
| Can a key create books? | **No.** A key is issued against an audiobook that already exists, after the author created it in the dashboard. Book creation stays a session-token action. |
| Expiry | Yes, as a settable field. `expires_at` is nullable and defaults to `NULL` (no expiry); an explicit `ttl_hours` at issuance populates it. Revocation is the control. |
| Who can issue? | Only `audiobooks.user_id` — the person who created that book — with a normal session token. No admin-issued-on-behalf-of, no "any moderator". |
| What turns it on? | **A per-book switch**, `audiobooks.author_api_enabled`, default `0`, flipped by that book's creator. Not a deployment flag: one book being on says nothing about any other. |

The first row is the one that changes the shape: **a key belongs to a book, not
to a person.** It cannot list books, create books, or reach any other book's
chapters. That also makes "who can issue" answerable without inventing a
permission model — the book's own `user_id` is the authority.

## What exists today

Four facts, all of which shape the design:

| Fact | Where | Consequence |
| --- | --- | --- |
| `media.hash` **is** the SHA-256 of the bytes, and the file path is derived from it | `persistence/media/hashing.rs`, `storage/media_key.rs` | Content addressing already exists. Nothing new is needed to *store* by hash. |
| Audio is **not** transcoded on upload | `persistence/audiobook/medium.rs` (only covers go through `convert_to_webp`) | The client's `sha256(file)` equals `media.hash` exactly. A hash computed on the author's machine is comparable to the server's. |
| The same bytes may be registered under many rows | `20260813120000_relax_media_hash_unique.sql` | A chapter always gets its own `media` row. Dedup is a *transfer* concern, not a *row* concern — which keeps `release_medium`'s reference counting intact. |
| `release_medium` will not unlink a file while another row shares its hash | `persistence/audiobook/release.rs` | Deleting one chapter can never break another chapter that happens to be the same bytes. |

And one fact about the existing key machinery that the new feature must not
repeat: `create_sync_key` / `list_sync_keys` / `revoke_sync_key` are written in
`handlers/sync.rs` but **mounted on no router** — `grep` finds no caller. Sync
keys can only be created by hand in the database today. Key management here
gets a route and a reachability test.

## Auth: one key, one book

```sql
CREATE TABLE author_api_keys (
    id INTEGER PRIMARY KEY,
    audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,      -- SHA-256 of the secret; the secret is shown once
    label TEXT NOT NULL DEFAULT '',
    hint TEXT NOT NULL,                   -- 'abk_3f9c…' for the dashboard list
    scopes TEXT NOT NULL DEFAULT 'read,write' CHECK (scopes IN ('read', 'read,write')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,                      -- NULL = no expiry (the default)
    revoked_at TEXT,
    last_used_at TEXT
);
CREATE INDEX idx_author_api_keys_audiobook ON author_api_keys (audiobook_id);
```

Prefix `abk_`, 32 random bytes hex — the same shape as `generate_sync_key()` in
`infrastructure/sync/mod.rs`, reused with a different prefix.

`user_id` is the book's creator, denormalized so the guard can build an identity
in one query. Both foreign keys cascade: deleting a book deletes its keys, so no
orphan credential can outlive the thing it unlocks.

**Expiry.** `expires_at` is written as RFC3339 when `ttl_hours` is supplied, and
left `NULL` otherwise. The guard rejects it only when it is non-null and in the
past; it parses both RFC3339 and SQLite's `CURRENT_TIMESTAMP` spelling, as
`sync_key_guard` already does. `NULL` intentionally means "until revoked" — an
author's import tool may run for months, and a key expiring halfway through a
900-chapter sync is worse than a key that lives until someone revokes it. The
dashboard shows `expires_at` and `last_used_at` so a forgotten key is visible.

**Scopes.** A `read` key can use the manifest and both download endpoints but
cannot change anything. That is the difference between a key an author gives to
their backup script and one they give to their publishing pipeline. The guard
maps `GET`/`HEAD` to `read` and everything else to `write`.

### Why not `sync_keys`

A sync key reads the whole database and every media file (`/sync/database`,
`/sync/media/{hash}`, `/sync/artifact/*`). An author key must reach exactly one
book. Sharing the table would mean the scope of a key is decided by which
endpoint it hits — one forgotten guard away from an author downloading the
production database. `sync_keys.mode` is also constrained to `'pull'` by a
`CHECK`, so it cannot express write access at all.

### The guard

`author_key_guard` in `web/api/middlewares/auth.rs`, a sibling of
`sync_key_guard`. One query joins the three tables:

1. `Authorization: Bearer abk_…` → hash → lookup on `token_hash`.
2. Reject revoked, expired, or insufficient scope for the method.
3. Reject with **404** when `audiobooks.author_api_enabled = 0` — the per-book
   switch. Off is indistinguishable from "no such book", so a key cannot be
   used to discover which books are exposed.
4. Reject when `author_api_keys.user_id != audiobooks.user_id` — a key issued to
   the creator stops working if the book is ever reassigned.
5. **Read `users.role` in the same query** and carry it in the marker. Authoring
   is a `mod`-tier action today; keys are strictly narrower than the session
   they were issued from, and re-reading the role means a demotion takes effect
   on the next request rather than at key expiry.
6. Bump `last_used_at` (best effort, as `sync_key_guard` does).
7. Insert `AuthorKeyAuth { key_id, audiobook_id, user_id, role, scopes }`.

Every handler then calls one `assert_key_scope(&auth, path_id)` helper at the
top. A mismatch answers **404, not 403**, so a key cannot be used to probe which
other books exist. One helper, not a check scattered across nine handlers.

### Issuance

`POST /author/audiobooks/id/{id}/keys` is guarded by `user_guard` + `mod_check`
(the same tier as the rest of authoring) and calls `assert_owned` on the book
before inserting. The response carries the raw secret **once**; `GET` on the
same path returns only `id`, `label`, `hint`, `scopes`, `created_at`,
`expires_at`, `revoked_at`, `last_used_at`. `DELETE …/keys/{key_id}` revokes,
scoped to the same book so one book's key management cannot revoke another's.

Key creation is session-guarded, not key-guarded — otherwise you would need a
key to get a key.

### What the handlers do with the identity

Nothing new, and that is the point. Every existing write command already takes
`user_id` and `is_admin`, and `assert_owned` already enforces ownership:

```rust
AddTrackCommand { user_id: auth.user_id, is_admin: false, .. }
```

So the key supplies an identity and the service layer does the authorization.
No parallel permission model, no second place to get it wrong.

## The protocol

Read the book, diff by hash, upload what is missing.

```
GET  /author/audiobooks/id/42          →  chapters with sha256 + size
        │
        ├─ local tool compares against its own files
        │
        └─ PUT /author/audiobooks/id/42/chapters/{n}   ←  only chapters that differ
                 200 unchanged | 200 replaced | 201 created
```

**The read is the plan.** There is no separate "plan" or "hash check" endpoint:
`GET /author/audiobooks/id/{id}` already carries every hash the diff needs, and
a second endpoint recomputing it would be a second thing to keep in sync. It is
also the only endpoint that can exist with a book-scoped key — a server-wide
"do you already have these bytes?" probe would answer questions about other
books, which is exactly what the key is not allowed to reveal.

The `200 unchanged` outcome is the safety net for retries and races (another
client uploaded the same chapter a second ago), not the main dedup path. The
main path is that the author's tool never sends those bytes at all.

## Endpoints

Key tier = `Authorization: Bearer abk_…`. JWT tier = a normal session token.
Every key-tier path carries the book id, and the guard asserts it matches the
key's book.

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/author/audiobooks/id/{id}` | key | Book + **every** chapter with `sha256`, `size`, `duration_seconds` |
| PATCH | `/author/audiobooks/id/{id}` | key | title / description / translator / tags (names) |
| POST | `/author/audiobooks/id/{id}/status` | key | `draft` / `published` / `archived` |
| PUT | `/author/audiobooks/id/{id}/chapters/{number}` | key | **Idempotent** upload of one chapter |
| PUT | `/author/audiobooks/id/{id}/chapters` | key | Batch chapter metadata (titles, durations, slots) |
| PATCH | `/author/audiobooks/id/{id}/chapters/{number}` | key | One chapter's title/duration |
| DELETE | `/author/audiobooks/id/{id}/chapters/{number}` | key | Remove a chapter |
| GET | `/author/audiobooks/id/{id}/chapters/{number}/audio` | key | Download the original bytes |
| GET | `/author/audiobooks/id/{id}/archive` | key | Batch download (zip, capped) |
| GET | `/author/audiobooks/id/{id}/settings` | JWT + owner | `{ enabled, key_count, active_key_count }` |
| PUT | `/author/audiobooks/id/{id}/settings` | JWT + owner | The per-book switch |
| POST | `/author/audiobooks/id/{id}/keys` | JWT + owner | Issue a key (secret returned once) |
| GET | `/author/audiobooks/id/{id}/keys` | JWT + owner | List this book's keys (never the secret) |
| DELETE | `/author/audiobooks/id/{id}/keys/{key_id}` | JWT + owner | Revoke |

No book-listing, no book-creation, no tag-vocabulary read. The tag vocabulary
is global, and metadata PATCH already accepts plain tag names (`replace_tags`
slugifies and reuses or creates rows), so a book-scoped key never needs it.

The two tiers live in one `routes()` function, split into two sub-routers and
`.merge`d — the pattern `handlers/audiobook/mod.rs` already uses for its mod and
public halves.

### Why chapters are addressed by number

`number` is the author's own slot, unique per book (`assert_slot_is_free`), and
stable across uploads. `track_id` is a database detail the author's tooling
never sees. Addressing by number is what makes `PUT` a real idempotent upsert
rather than an insert-with-a-check.

## Upload semantics

`PUT /author/audiobooks/id/{id}/chapters/{number}` — raw body, no multipart:

```
Content-Type: audio/mpeg
X-Content-SHA256: b4c0ffee…            (optional but recommended)
?title=Ch%C6%B0%C6%A1ng%2017&duration_seconds=2413
```

The title travels in the query string rather than a header because chapter
titles are Vietnamese here and non-ASCII headers are a portability minefield.
The dashboard's existing multipart route is untouched.

| Server state | Body outcome | HTTP | Writes |
| --- | --- | --- | --- |
| No chapter at `number` | created | `201` | file (if absent) + `media` row + `audiobook_tracks` row |
| Chapter at `number`, same hash | unchanged | `200` | **nothing** |
| Chapter at `number`, different hash | replaced | `200` | file + new `media` row, old one released |

`X-Content-SHA256` is verified against the bytes actually received. A mismatch
is `422` and nothing is written — that is the "uploading correctly" half of the
requirement: a truncated or corrupted transfer is caught at the door instead of
becoming a chapter that plays for eleven seconds.

### Where the skip has to happen

This is the one place where the obvious implementation is wrong.

`store_medium` (`persistence/audiobook/medium.rs`) already skips the disk write
when the file exists (`wrote_file = !try_exists`). It then unconditionally calls
`write_chunked_audio`, and `chunked::write` has **no early exit** — it
re-compresses every chunk from scratch (`storage/chunked.rs:129`). So a
"unchanged" upload routed through the normal write path would spend full zstd
CPU on 40 MB of audio to produce files that already exist.

Therefore the same-hash check must short-circuit in the **handler**, before any
service call: resolve chapter by number, compare `media.hash`, return. The
service layer never sees the request.

### Verifying without hashing twice

`store_medium` computes the hash internally and throws it away (it returns only
`media_id`). Change it to return `(media_id, hash)` — a one-line signature
change with two call sites — so the handler compares against the digest the
write path already computed instead of running SHA-256 over 40 MB a second time
on the blocking pool.

## Downloads

**Single.** `GET …/chapters/{number}/audio` resolves the chapter's `media` row,
derives the key with `media_key`, and streams the original bytes — bucket first
when `AUDIO_BACKEND=r2`, disk otherwise. This is the same resolution as
`get_media_by_hash` in `handlers/sync.rs`, which should be factored into one
helper rather than copied.

Headers: `Content-Type`, `Content-Length`, `X-Content-SHA256`, and
`Content-Disposition: attachment; filename="017 - Chapter title.mp3"` so a batch
of curls lands as a readable directory instead of `abt-42-b4c0ffee-7f3a.mp3`.
The public `/media/i/{short_name}` route cannot do that last part, which is the
reason this is a keyed endpoint and not a redirect to the existing one.

Range/`206` is **not** in v1. `handlers/media/serving.rs` already implements it
if resume turns out to matter for 40 MB chapters.

**Batch.** `GET …/archive?numbers=1-50,53` builds a zip in a temp file on the
blocking pool and streams it, deleting on completion — the `stream_temp_file`
pattern from `handlers/sync.rs`, which already handles the client-disconnect
case. `zip` is a dependency (0.6) but only used for *reading* today
(`support/demo_archive.rs`); writing is new code, ~60 lines.

The cap matters: a 900-chapter book is ~30 GB and a temp file of that size will
fill the VM's disk (which already runs close to full). `AUTHOR_ARCHIVE_MAX_BYTES`
defaults to 2 GiB; above it the endpoint answers `413` naming `?numbers=`. For
anything larger the author loops the single-chapter endpoint — which is the same
code path, so there is no second implementation to maintain.

## Metadata reads

`GET /author/audiobooks/id/{id}` is the manifest, and it is the one read that
must **not** be windowed by default. The public feed windows chapters
(`tracks_limit`, max 200) because a player only needs a screenful; an author
diffing a book needs every hash or the diff is silently partial. Windowing stays
available as an explicit opt-in, with `has_more_chapters` on every answer so a
truncated list can never be mistaken for a short book.

Chapter entries carry: `number`, `title`, `duration_seconds`, `sha256`, `size`,
`file_type`, `updated_at`, `play_count`. Book-level metadata is the existing
`AudiobookDetails` shape minus the public URL rewriting.

## What this deliberately does not do

- **No giant batch multipart.** 50 chapters in one request is 2 GB of body
  against a `DefaultBodyLimit` of 100 MB and a handler that buffers the whole
  thing in memory (`Bytes`). Batch upload is the diff plus a loop, and a batch
  *metadata* endpoint for the parts that are cheap.
- **No "claim by hash" endpoint.** The server could, in principle, let a client
  declare a hash it already stores elsewhere and create a chapter without any
  bytes crossing the wire. It is out of v1 because `chunked::write` needs the
  raw bytes to build the zstd manifest, and the only copy may be in R2 — that
  turns a cheap link into a download-and-recompress operation. Worth revisiting
  once chunks are derived from the stored file rather than the upload buffer.
- **No shared `media` rows.** One row per chapter, as today. Sharing rows would
  mean two chapters fighting over one `release_medium` refcount.
- **No streaming body handling in v1.** The 100 MB ceiling stays, matching the
  dashboard. Chapters above it need the resumable session pattern from
  `handlers/v86/upload_session.rs`, and `chunked::write` takes `&[u8]` anyway —
  that is a separate change, not a hidden assumption in this one.
- **No cover upload.** Covers are an image path with `convert_to_webp`; a script
  driving chapters does not need it, and it keeps the first version to one body
  shape.

## Change list

| File | Change |
| --- | --- |
| `migrations/20261115000000_create_author_api_keys.sql` | new table, book-scoped |
| `domain/errors/author.rs` | `AuthorError` (401 invalid/expired key, 403 wrong scope, 413 archive too large, 422 hash mismatch) |
| `domain/errors/mod.rs` | register `pub mod author;` |
| `web/api/middlewares/auth.rs` | `author_key_guard` + `AuthorKeyAuth` |
| `web/api/handlers/author/mod.rs` | route tables (key tier + JWT tier), re-exports, `assert_key_scope` |
| `web/api/handlers/author/{dto,response,read,write,download,keys}.rs` | handlers |
| `web/api/router.rs` | `.nest("/author", handlers::author::routes(state.clone()))` |
| `persistence/audiobook/keys.rs` | key CRUD (`issue`, `list`, `revoke`, `authenticate`) |
| `persistence/audiobook/medium.rs` | `store_medium` returns `(media_id, hash)` |
| `persistence/audiobook/tracks.rs` | `resolve_chapter_by_number`, `upsert_chapter_by_number` (resolve then delegate to `add_track` / `replace_track_medium`) |
| `persistence/media/store.rs` | `chunked::write` early exit when a manifest already exists |
| `application/services/audiobook.rs` | three new trait methods |
| `frontend/src/routes/dashboard/audiobooks/id/[audiobook_id]/` | small "API keys" panel: issue, list, revoke |

The `upsert_chapter_by_number` shape is worth stating plainly: it resolves
`number → track_id` and then calls the existing `add_track` /
`replace_track_medium`. No new write path, so the slot-free assertion, the
audio-type validation, the bucket mirror, the chunk writer and the
`release_medium` accounting all keep working exactly as they do for the
dashboard.

## Tests

- A key for book A cannot read, write, or delete book B — `404`, and book B is
  unchanged. This is the test that matters.
- A key is rejected when `audiobooks.user_id` no longer matches its issuer.
- Only the book's creator can issue a key for it; another moderator gets `403`.
- Revoked key → `401`; key with `expires_at` in the past → `401`; key with
  `expires_at = NULL` → accepted indefinitely. A `read` key can `GET` but gets
  `403` on `PUT`.
- Role removed from the owner → key stops authoring on the next request.
- `PUT` with a mismatched `X-Content-SHA256` → `422`, no `media` row, no file,
  no track.
- `PUT` twice with the same bytes → second is `200 unchanged`, `media` row count
  and `chunked` manifest mtime both unchanged. (This is the test that catches
  the recompression trap.)
- `PUT` with new bytes over an existing chapter → `200 replaced`, old `media`
  row gone, old file gone *only* if no other row shares its hash.
- `archive` above the cap → `413` naming `?numbers=`.
- Key-management routes are **reachable** — one `oneshot` per route, because the
  sync-key precedent in this repo is three handlers that nobody can call.

## Docs

- `docs/reference/api-rest.md` — new `/author` section. The file's own header
  says a route without an entry is an incomplete change.
- `docs/reference/auth-and-roles.md` — new `author-key` tier in the guard table,
  and an "Author keys" section beside "Sync keys" explaining why they are not
  the same thing.

## Implementation notes (2026-11-15)

What shipped differs from the plan above in these ways, all deliberate:

- **A deployment-wide switch was added first, then removed.** The initial
  implementation gated the whole router behind `AUTHOR_API_ENABLED`. That was
  wrong: exposure is per book. The flag is gone; `audiobooks.author_api_enabled`
  (default `0`) is the only switch, and `GET /author/status` no longer exists.
  The guard answers 404 for a book whose switch is off, so a valid key cannot
  be used to discover which books are exposed.
- **`store_medium` still hashes twice.** The plan proposed returning
  `(media_id, hash)` to avoid a second SHA-256 pass. It was dropped: the digest
  is needed *before* the write path runs (to verify the header and to short-
  circuit an unchanged upload), so returning it afterwards does not help. The
  duplicate pass is cheap next to the zstd encode it saves.
- **`scopes` landed as `read` / `read,write`.** A read-only key can read the
  manifest and download, but not write. The guard maps GET/HEAD to `read`.
- **Batch chapter metadata is title/duration only.** Moving a slot stays on the
  single-chapter PATCH, because two chapters claiming one number in one request
  would be ambiguous.
- **`chunked::write` still has no early exit.** Instead of changing it, the
  handler short-circuits before any service call. The encoder is untouched, so
  the dashboard's upload path behaves exactly as before.
- **The archive reads the disk copy.** The plain file is authoritative and kept
  even for chunked rows, so no bucket fetch is needed inside the blocking zip
  writer.
- **Tests.** Four route-reachability tests drive the real router with `oneshot`
  (a missing route would answer 404, not 401), plus a test that runs the actual
  chapter queries against a migrated in-memory database — that one caught
  `AND t.number = ?` being appended after `ORDER BY`, which no route test would
  have seen.
