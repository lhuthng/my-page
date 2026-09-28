# Operations

Audience: whoever backs up, restores, or moves environments. Update trigger:
a change to backup, sync, or storage-migration procedures.

## Backups (VM cron)

`blog/backend/backup.sh` compresses `data/`, `media/`, and `project-demos/`
into dated tarballs and pushes them to an rclone remote
(`r2-backup:blog-backup/daily-backups`).

Setup:

```bash
sudo apt install rclone -y
mkdir -p ~/.config/rclone/
nano ~/.config/rclone/rclone.conf   # remote name: r2-backup, bucket: blog-backup
```

Optional daily cron:

```bash
crontab -e
# 0 0 * * * cd ~/MyPage/blog/backend && ./backup.sh
```

## Sync keys (prod → dev pull)

The dashboard's Backup & Sync page (admin only) issues a short-lived sync key
(`bsk_…`, shown once, stored hashed, revocable). A dev machine then pulls the
whole environment:

```bash
cd blog/backend
cargo run --bin sync-pull -- --url https://huuthangle.site --key @sync.key
```

The pull fetches a manifest, streams a consistent SQLite snapshot (previous
database kept aside as `*.pre-sync-*`), rewrites stored URLs for the local
`MEDIA_PATH`/`PROJECT_DEMOS_PATH`, and downloads media, demo files, and game
artifacts — skipping files already present with the right size, so an
interrupted sync can simply be re-run. `--prune` removes local files absent
from the source; `--skip media|demos|artifacts`, `--dry-run`, and `--yes`
skip the interactive prompt (the overwrite warning prints on every run).

Security: a sync key grants full read access to the database (including
password hashes) and every file — keep the TTL short, label keys, revoke
after use. The `/sync` API is pull-only by design (`sync_keys.mode` only
allows `'pull'`); a dev → prod push is deliberately not implemented.

## v86 artifacts: R2 ↔ fs migration

v86 artifacts (system IMG chunks, game disks, ISOs, snapshots, saves) go
through the `ObjectStore` abstraction. To switch production from R2 to fs:

> **Unresolved, and it blocks this procedure as written.** Steps 2 and 5 drive R2
> with the `aws` CLI. This repo has no `aws` CLI dependency — nothing installs
> it, [deployment.md](deployment.md) lists only Docker, Compose and nginx on the
> VM, and the one R2 command-line tool the project actually uses is `rclone`
> (`backup.sh`). So neither script can run on the VM today. They need moving onto
> rclone before this section is trustworthy. Flagged rather than fixed, because
> it is a separate change from the audiobook move.

1. Deploy the new backend image (still `auto` = R2 mode).
2. Run the idempotent backfill on the VM (content-addressed keys):
   `cd ~/MyPage/blog/backend && ./sync_r2_to_fs.sh`
3. Set `STORAGE_BACKEND=fs` in `backend/.env`, then
   `docker compose up -d backend`.
4. Verify a v86 game boots and a chunk URL resolves:
   `curl -sI https://huuthangle.site/v86/assets/systems/<sha256>/<part> | head -1`
5. Reclaim R2 storage (deliberately manual):
   `aws s3 rm --recursive s3://$R2_BUCKET/v86/`

Key layout and serving behavior:
[../reference/media-and-storage.md](../reference/media-and-storage.md).

## Audiobook audio: disk → bucket

Audiobook audio shares the v86 artifact bucket: audio under the `audio/` prefix,
artifacts under `v86/`. Nothing to create and no new credentials — the `R2_*`
block already in `backend/.env` is all of it. The one thing that must be set is
`R2_PUBLIC_URL`, because that is where audio is served from.

Only audiobook audio is affected — precisely, the media an `audiobook_tracks`
row points at. Images, covers, avatars, video and models stay on `MEDIA_PATH`
and are never mirrored, so this procedure cannot disturb them. Audio that no
track plays (a media-library sound file, or media left behind by a track
replacement) is counted in the report and deliberately left on disk.

The object key is the disk key with the prefix, so the two stores are
interchangeable and the copy is a plain one — **no file moves on disk**. Reads
are bucket-first with a disk fallback, so for as long as the disk copy is there,
anything the bucket does not have yet still serves.

**Nothing has to move, and there is no data migration.** No schema change and no
rewrite of any row: the `media` rows already hold everything the object key is
derived from, and the stored `url` column is only ever a *disk* fallback, never
consulted for a bucket read. Every step below only ever *copies*, and each one is
optional:

| You do this | Existing library | New track uploads | Disk copies |
| --- | --- | --- | --- |
| ship the build, change no env | disk | disk | untouched |
| + `AUDIO_BACKEND=r2` | **still disk**, served from disk | bucket | untouched |
| + `./backfill-audio.sh --upload` | bucket | bucket | untouched |
| + delete what the report names | bucket | bucket | gone (deliberate) |

**Stopping after row 2 is a valid end state, not a half-finished migration.** A
track the bucket does not have keeps being served from disk. Running the backfill
is what puts the *existing* library in the bucket; skipping it costs nothing but
disk space.

The order matters for the steps you do take — the build goes out while
`AUDIO_BACKEND` is still `fs`, so the first thing that happens on the VM is a
report, not a cutover.

1. Add to the VM's `blog/backend/.env`. The deploy pipeline ships only
   `docker-compose.yml` and `ops/nginx/*`, so this file is edited by hand:

   ```bash
   AUDIO_BACKEND=fs     # still fs — this deploy changes nothing
   ```

   `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET` and
   `R2_PUBLIC_URL` are already there for the artifacts. If `R2_PUBLIC_URL` is
   not, set it — the bucket's public domain, e.g.
   `https://disk.huuthangle.site` — because `AUDIO_BACKEND=r2` will not start
   without it.

2. Deploy the backend and restart it. The startup line says which mode it came
   up in:

   ```
   Audio backend: r2 (bucket retro-games, prefix audio/, public https://disk.huuthangle.site)
   ```

   `docker compose logs backend | grep 'Audio backend'`. An incomplete `R2_*`
   set or a missing `R2_PUBLIC_URL` is a container that will not start, rather
   than a silent fallback to disk.

3. Backfill. On the VM, with the stack up. The wrapper is not shipped by the
   deploy, so copy it over once first — the command is further down — then:

   ```bash
   cd ~/MyPage/blog/backend
   ./backfill-audio.sh              # report + reconcile, no writes
   ./backfill-audio.sh --upload     # copy the objects the bucket is missing
   ```

   The script is a `docker compose exec` and nothing else. The reconcile is
   `backfill-audio`, which ships in the backend image, so it runs with the app's
   own `R2_*` credentials, `MEDIA_PATH`, database and `storage::media_key` —
   one implementation of the key layout rather than a shell copy of it to keep
   in step. It needs **no CLI installed on the VM**: the image is built in CI
   (`docker/build-push-action`) and the VM only pulls it. In particular it does
   not shell out to `aws` — this repo has no `aws` CLI dependency.

   The deploy pipeline deliberately does **not** ship the wrapper, so copy it
   over yourself, reusing the credentials `scripts/emergency/deploy.env` already
   holds (`VM_DEPLOY_PATH` is the absolute path to the directory holding
   `blog/docker-compose.yml` on the VM, i.e. `.../MyPage/blog`):

   ```bash
   set -a; source scripts/emergency/deploy.env; set +a
   scp -i "$VM_SSH_KEY" blog/backend/backfill-audio.sh \
     "$VM_USER@$VM_HOST:$VM_DEPLOY_PATH/backend/"
   ssh -i "$VM_SSH_KEY" "$VM_USER@$VM_HOST" \
     "chmod +x $VM_DEPLOY_PATH/backend/backfill-audio.sh"
   ```

   Only the wrapper needs copying. The reconcile itself is in the image, which
   the deploy already pulled.

   From a dev checkout, `cargo run --bin backfill-audio [-- --upload]` is the
   same reconcile against a `sync-pull`ed copy.

   It does **not** need `AUDIO_BACKEND=r2`. It addresses the bucket through the
   `R2_*` variables directly, which is exactly what lets you fill the bucket
   while the app is still serving audio from disk — do not flip the switch first.

   It only ever reads and writes the `audio/` prefix and only ever considers
   media an `audiobook_tracks` row points at, so the v86 artifacts sharing the
   bucket and every image row are left alone. It exits non-zero when a track's
   bytes are not recoverable from the bucket, so it gates the cutover on its
   own. Expect *audio nothing plays* to be non-empty and harmless: it is counted
   and reported, never uploaded. *No copy on disk and none in the bucket* is the
   same — a row whose file was already gone before any of this. *In bucket but
   no track* should be empty, because audio is never replaced in place and never
   deleted; a non-empty list means a manual `aws s3 rm` or an interrupted
   upload, and the objects it names can be deleted. *Wrong size in bucket* is
   the one that means a copy landed badly; `--upload` overwrites it.

4. Flip the mode and restart:

   ```bash
   sed -i 's/^AUDIO_BACKEND=fs/AUDIO_BACKEND=r2/' backend/.env
   docker compose up -d backend
   ```

5. Verify. The redirect is the thing to look at first, because it is invisible
   from the backend's own logs:

   ```bash
   curl -sI https://api.huuthangle.site/media/i/<short_name> | head -1
   # HTTP/2 302 — and a `location:` on the public domain

   curl -sI "$(curl -s -o /dev/null -w '%{redirect_url}' \
     https://api.huuthangle.site/media/i/<short_name>)" | head -6
   # 200, `content-type: audio/mpeg` (not application/octet-stream),
   # `cache-control: public, max-age=31536000, immutable`

   curl -s -r 0-99 -o /dev/null -w '%{http_code} %{size_download}\n' \
     "https://api.huuthangle.site/media/i/<short_name>"   # 206 100
   ```

   Then play a chapter in the macOS app and **seek** — AVPlayer is the strictest
   client here.

**What to expect from step 5, and what not to.** The macOS app asks
`api.huuthangle.site` and follows the redirect itself, so its audio goes
straight from the bucket to the device and never crosses the VM. The web blog's
audiobook player does not: `fixUrl` hands `<audio>` a same-origin
`/api/media/i/...`, so the SvelteKit Node process fetches the backend, follows
the 302, and streams the bytes back — the VM still carries a first play. Repeat
plays are served from the browser's one-year `immutable` cache, so it is the
first play that costs. Blog images and covers do take the direct path in
production, because `BACKEND_ORIGIN` is set for the frontend and
`fixClientRoute` hands the browser an absolute backend URL.

**The bytes are public.** `R2_PUBLIC_URL` serves the whole bucket, so every
object is fetchable at `https://disk.huuthangle.site/audio/...`. The keys are
sha256, so objects are unlisted rather than open — but that is not the same as
private, and a prefix cannot be made private inside a public bucket without a
Worker in front of it. Nothing on this path is signed.

**Rollback** is `AUDIO_BACKEND=fs` and a restart. Nothing in this procedure
writes to or deletes the disk tree — the bucket-first read only ever *adds* a
source — so the fallback is always the state you started from.

**Reclaiming disk space is a separate, deliberate decision.** The disk copy is
load-bearing until `backfill-audio.sh` exits 0 and prints that every track's audio
is in the bucket at the right size, and it is the only fallback if the bucket
credentials break. Two things to settle before deleting it:

- `backup.sh` tars `media/` into `r2-backup:blog-backup`. With the tree gone,
  the backup carries no audio and the bucket becomes the only copy — so it needs
  versioning or a lifecycle rule of its own first. The `audio/` prefix is where
  an audio-specific rule would go.
- Only track audio is in the bucket. Deleting `media/` wholesale would also
  delete every image, cover and avatar, which are disk-only and have no bucket
  copy at all. Delete the track files the report names, not the tree.
- Nothing in the code deletes the disk tree, on purpose. If it is deleted, do it
  by hand.

## Restore

Backups are dated tarballs of the three persistent directories. Restore =
extract back into `blog/backend/{data,media,project-demos}` and restart the
backend (migrations are already up to date inside the database). nginx on the
VM must route `/v86/`, `/games/s/`, and `/project-demos/` to the backend in
`fs` mode.

With `AUDIO_BACKEND=r2` the tarball's `media/` is a partial copy — whatever was
on disk when the backup ran — and the bucket holds the rest of the audio. A
restore is then the database plus the bucket as it stands, and the disk tree only
has to be present, not complete. Images and the other disk-only media are always
fully in the tarball, because they were never in the bucket.
