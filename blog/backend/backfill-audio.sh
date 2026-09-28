#!/usr/bin/env bash
#
# Backfill and reconcile the audiobook audio bucket against the database.
#
# A thin wrapper. The reconcile itself is `backfill-audio`, a small binary that
# ships inside the backend image, so it runs where the app runs: same `R2_*`
# credentials, same `MEDIA_PATH`, same database, same `storage::media_key` the
# write path uses. That is deliberate — a second implementation of the key
# derivation in shell is a second thing to get wrong, and the extension for
# `audio/mpeg` is `.mp3`, not `.mpeg`.
#
# **Nothing is installed on the VM for this and nothing is built there.** The
# image is built in CI (`docker/build-push-action`) and the VM only pulls it,
# and all this needs is `docker compose`, which the VM already runs. There is
# deliberately no `aws` CLI dependency: this repo does not use one.
#
# Usage:
#   cd ~/MyPage/blog/backend
#   ./backfill-audio.sh                # report + reconcile, no writes
#   ./backfill-audio.sh --upload       # copy the objects the bucket is missing
#
# Options are the binary's, passed straight through:
#   --upload            write the missing / wrong-size objects (default: report)
#   --env <path>        .env holding DATABASE_URL / MEDIA_PATH / R2_*
#   --media-dir <path>  override MEDIA_PATH
#   -h, --help          the binary's own help
#
# The stack has to be up (`docker compose up -d`). The backfill only reads the
# database and the media tree and only ever writes to the bucket, so running it
# against a live stack is safe. It does **not** need `AUDIO_BACKEND=r2` — it
# addresses the bucket through `R2_*` directly, which is the whole point: it
# fills the bucket *before* the serving switch is flipped.
#
# Exits non-zero when a track's audio is not recoverable from the bucket, so it
# gates the cutover on its own.
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# The compose file lives one level up from this script. Passing it with -f also
# makes compose resolve the service's relative bind mounts against its own
# directory, so the container sees the same ./backend/{data,media} either way.
COMPOSE_FILE="$BASE_DIR/../docker-compose.yml"

if ! command -v docker >/dev/null 2>&1; then
	printf 'backfill-audio: docker is not installed\n' >&2
	exit 1
fi
if [[ ! -f "$COMPOSE_FILE" ]]; then
	printf 'backfill-audio: no compose file at %s\n' "$COMPOSE_FILE" >&2
	exit 1
fi

# `exec` so the binary's exit status is this script's, and `-T` so it works when
# there is no TTY (a cron entry, or a deploy step).
exec docker compose -f "$COMPOSE_FILE" exec -T backend backfill-audio "$@"
