# Collapsing the two body columns (`content` + `draft`)

Status: proposal. Nothing implemented.
Companion to `editor-feedback-ux.md`.

## 1. The question

`posts.draft` was introduced as "the draft body". It has since become the
*primary* editing surface, while `content` is a snapshot written only at
publish. The word "draft" now names four different things:

| Name | What it actually is |
| --- | --- |
| `posts.draft` | the body you always edit |
| `posts.status = 'draft'` | the row is not public |
| `editor-draft:*` (browser storage) | local crash recovery |
| "Ver. Draft / Ver. Published" (UI) | which column the textarea is bound to |

The body column is the misnamed one. It is not a draft — it is *the* body.

## 2. Evidence that it is redundant

Measured against the local database (`backend/data/blog.db`, read-only):

- 19 rows, all `status = 'published'`. Zero drafts.
- **19 of 19 rows have `content` byte-identical to `draft`.** Bodies run up to
  20 KB; this is not a case of two empty strings.
- Zero rows where `content IS NULL`. Zero where `draft IS NULL`.

So across the entire dataset the two columns have **never once diverged**. The
staging capability has never been exercised, and every body is stored twice.

There is a second, structural problem. `draft` is the **only** staged field:

- `title`, `slug`, `excerpt`, `tags` are written straight to the live row, so
  editing them on a published post changes the public page on save.
- The body goes to `draft` and stays invisible until publish.

That inconsistency — not the column count — is most likely the "a little off"
feeling. Metadata publishes immediately; the body does not.

## 3. Proposed change

One body. `status` becomes the only draft axis.

- Keep `content`. Drop `draft`.
- `publish` reduces to a pure state flip: `status = 'published'`, and
  `published_at` set only on the first publish (unchanged semantics).
- The editor binds a single body, always editable. The version toggle and the
  read-only "published" view go away.
- `with_draft` query parameter and the `draft` field in responses go away.
- `reading_time_minutes` is recomputed when the body is saved, rather than at
  publish time.
- `validate_body`'s "content or draft" comment in `helper/string.rs` collapses
  to one argument.

Net effect: **save = live, publish = visibility.** The same rule already
governs every other field.

## 4. Migration

Ordered so the column is dropped last and the change is reversible until then.

1. **Verify** no divergence in the target database (must return 0):
   ```sql
   SELECT COUNT(*) FROM posts
   WHERE COALESCE(content,'') <> COALESCE(draft,'');
   ```
   If non-zero, stop: those rows hold unpublished body edits and a human has to
   decide whether the draft or the published text wins.
2. **Backfill** anything never published:
   ```sql
   UPDATE posts SET content = draft WHERE content IS NULL;
   ```
   No-op on the current data, but the production database may differ from this
   local copy.
3. **Ship the code** that reads and writes `content` only. Deploy this *before*
   dropping the column, so the old column still exists if a rollback is needed.
4. **Drop the column** in a follow-up migration:
   ```sql
   ALTER TABLE posts DROP COLUMN draft;
   ```
   Requires SQLite 3.35+. `draft` is not indexed and not referenced by a
   partial index, so a plain `DROP COLUMN` should work; if the runtime SQLite is
   older, the table needs the usual rebuild (create new, copy, rename).
5. **Remove** the now-dead `with_draft` parameter and `draft` response fields.

## 5. Touchpoints

Backend:

- `src/domain/entities/{post,project,game}.rs` — drop the `draft` field.
- `src/infrastructure/persistence/{post,project,game}.rs` — the `INSERT` that
  writes `draft`, the `SELECT` lists, and the `publish` copy.
- `src/infrastructure/web/api/handlers/{post,project,game}.rs` — the `draft`
  request/response fields, `with_draft`, and the
  `content.xor(draft)` mutual-exclusion check (which only exists because two
  body fields exist).
- `src/infrastructure/web/graphql/{rows,resolvers,types}.rs` — `draft` in the
  admin schema.
- `src/helper/string.rs` — comment on `validate_body`.

Frontend:

- `lib/features/editor/model/state.js` — `bodies: { draft, content }` becomes a
  single `body`.
- `lib/features/editor/model/diff.js` — `buildPatch` currently diffs only
  `bodies.draft` and then sends *both* fields. Becomes one field, one diff.
- `lib/features/editor/view-model/create-editor-vm.svelte.js` — `ui.view`,
  `forDraft`, `activeBodyKey`, `toggleVersion`; the `bothBodies` arrays in the
  three save paths.
- `lib/components/editor/EditorToolbar.svelte` — remove the version toggle.
- `lib/components/editor/PostEditorShell.svelte` — `disabled={!vm.forDraft}`
  becomes always editable.
- `routes/dashboard/{posts,projects,games}/id/*/+page.svelte` — `draft: data.draft`
  mapping.
- `routes/dashboard/{posts,projects,games}/id/*/+page.server.js` —
  `decodeShortNames(data.draft, ...)`.
- `lib/features/editor/tests/{state,diff}.test.js` — fixtures assert
  `{ draft, content }`.

## 6. Risks

- **Behavioral change on live posts.** After this, saving the body of a
  published post publishes the change immediately. That is the point, but it
  deserves a signal: label the button "Save and update live" on a published
  entry rather than a bare "Save".
- **Loss of staging.** If you ever want to draft a rewrite of a live post
  without readers seeing it, this removes that ability. That is the one real
  trade-off, and the data says it has never been used.
- **Rollback** is clean while `draft` still exists (steps 1-3). Once step 4
  runs, recovery needs the backup tarball.

## 7. Alternative, if staging is wanted

Keep two bodies but rename so the roles are honest, and make the split earn its
keep:

- `content` → `published_body`; `draft` → `body`.
- Editor: `bodies.published` / `bodies.working`; the toggle becomes an
  explicit "editing / live" indicator rather than a version switch.
- Publishing shows a diff of what is about to go live.

Without that diff, the two-column split is invisible to the user and only adds
a way to be confused — which is the situation today.

## 8. Implementation record

Shipped on 2026-09-11. Differences from the plan above, and the things worth
knowing if this is ever revisited:

**Migration is one file, not two.** Steps 3 and 4 were merged into
`migrations/20260911000000_collapse_post_bodies.sql`. The backfill, a
divergence guard, and the `DROP COLUMN` all run in a single transaction, so
there is no window where the code expects one schema and the database has
another. The guard is a throwaway table with a named `CHECK` constraint —
SQLite has no `RAISE` outside triggers, so this is how the migration aborts
with a readable message instead of silently discarding an unpublished body:

```
CHECK constraint failed: divergent_post_bodies_would_be_lost
```

Verified against a copy of the real database: content length preserved exactly
(147,434 chars before and after), column dropped, other columns intact.
Verified the guard fires by injecting a divergent body — migration aborts and
the column survives.

**`reading_time_minutes` follows the save, not the publish.** It is now
recomputed in `update_post` from the incoming `content`. Because
`buildPatch` only sends `content` when the body actually changed, a metadata-only
save leaves reading time untouched.

**The editor gained a migration path for browser-stored drafts.** Local drafts
written by the old build have a `draft` key; the new build reads
`stored?.body ?? stored?.draft`, so an in-flight crash recovery is not lost on
deploy. This shim can be deleted once no old local drafts can plausibly exist.

**`as_id` survived; `with_draft` did not.** `with_draft` was doing double duty —
it gated the unpublished body *and* acted as the admin bypass for the owner
check on `get_*_by_slug`. With one body there is nothing to gate, so `with_draft`
is gone; the owner check is now derived plainly from `opt_claims`. Note this
means an authenticated owner request is no longer distinguishable from a public
one at the query layer, which is fine now but would matter if per-viewer fields
are added later.

**Untouched on purpose:** `total_drafts` in `dashboard.rs` and the GraphQL
stats resolver still count `status = 'draft'`. That is the publish axis, which
this change deliberately leaves alone. The `draft` variable inside
`MediaEditForm.svelte`, `MediaUploaderForm.svelte`, `AliasList.svelte` and
`EditorMediumEntity.svelte` is an unrelated local form-state name.

## 9. Rollout

**The migration runs by itself.** `.github/workflows/deploy.yml` notes that
migrations are applied by `sqlx::migrate!()` when the backend boots, and the
deploy blocks on healthchecks rather than a fixed sleep. So merging to `master`
is what applies `20260911000000_collapse_post_bodies.sql` to production — there
is no separate migration step to run, and no window to abort after the fact.

Before merging, run the step-1 divergence check against **production** (the
local copy proves nothing about the server):

```sql
SELECT COUNT(*) FROM posts
WHERE COALESCE(content,'') <> COALESCE(draft,'');
```

- `0` → the migration is a no-op backfill plus a column drop. Safe.
- non-zero → **stop**. Those rows hold unpublished body edits. The migration's
  guard will abort the deploy (no data is lost, but the backend will not start
  until the bodies are reconciled), so resolving them is a decision, not an
  accident.

If you would rather not deploy at all, put `[manual deploy]` in the commit
message — that skips the whole pipeline.

**Two side notes.** The workflow triggers on `blog/**`, so this `blog/docs/`
directory triggers a full production rebuild of both images; move docs out of
`blog/` if that is unwanted. And the local `frontend/build/` directory does not
matter either way — `frontend/.dockerignore` excludes it, and both deploy paths
build inside Docker from `blog/frontend/Dockerfile`.

## 10. Required production step: refresh a migration checksum

This is a **manual step that must happen before the next deploy**, and it is
unrelated to the body collapse. It exists because fixing a separate bug
required editing a migration that production had already applied.

**The bug.** `20260910010000_add_variant_columns_to_snapshot_upload_sessions.sql`
re-added `iso_sha256` to `game_v86_snapshot_upload_sessions`, but
`20260820010000_create_games.sql` — the only migration that creates that table —
already declares the column. On any database in the pre-migration state the
second `ALTER` fails with `duplicate column name: iso_sha256` and aborts the
chain at that version. Production happened to record it as applied (its stored
DDL shows `variant_index` appended by the `ALTER` while `iso_sha256` sits in the
`CREATE` body), so production was fine — but **fresh databases and the local dev
database could not migrate at all**, which is why `cargo test` could not run.

**The fix.** The redundant statement was removed from that migration. Verified
against a copy of the dev database: it now applies cleanly, adds `variant_index`,
and leaves `iso_sha256` untouched.

**The consequence.** sqlx validates a checksum for every applied migration.
Editing an applied file makes the recorded checksum stale, and sqlx then refuses
to start the backend with `VersionMismatch`. Production currently records the
*old* checksum, which still matches the *old* file — so the recorded value must
be updated to match the new file before deploying:

```sql
UPDATE _sqlx_migrations
SET checksum = X'111A1188B9748078EF4E4D148EEB15B2B542F83DCD28E6B50738763DCAB2403CE893549245B86739C9CE4C151A83DEB4'
WHERE version = 20260910010000;
```

Against `~/MyPage/blog/backend/data/blog.db` on the VM (host `sqlite3` is
present; the container mounts that file). To re-derive the value if the file is
ever touched again:

```bash
shasum -a 384 20260910010000_add_variant_columns_to_snapshot_upload_sessions.sql
```

That `shasum` output is exactly what sqlx stores — confirmed by matching the
current committed file against production's recorded checksum byte for byte
(`5FC769BD…63A5`), so no separate checksum tool is needed.

**Ordering.** Edit the file → update the checksum on the VM → deploy. If the
deploy runs first, the backend will not boot; run the `UPDATE`, then
`docker compose restart backend`. Deploying with `[manual deploy]` in the commit
message skips the pipeline entirely if you want to do this deliberately.

One thing this does **not** explain: production's run recorded success with a
checksum matching a file that fails on every other database in that state. Its
`variant_index` was appended by the `ALTER`, so the statement did execute. The
most likely reading is that the second statement was not executed on that run,
but that could not be reproduced locally and would need production access to
settle. It does not affect the fix, which is correct regardless of how the
original run behaved.
