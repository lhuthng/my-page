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

## R2 ↔ fs storage migration

v86 artifacts (system IMG chunks, game disks, ISOs, snapshots, saves) go
through the `ObjectStore` abstraction. To switch production from R2 to fs:

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

## Restore

Backups are dated tarballs of the three persistent directories. Restore =
extract back into `blog/backend/{data,media,project-demos}` and restart the
backend (migrations are already up to date inside the database). nginx on the
VM must route `/v86/`, `/games/s/`, and `/project-demos/` to the backend in
`fs` mode.
