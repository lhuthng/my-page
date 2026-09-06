#!/usr/bin/env bash
#
# Backfill v86 artifacts from Cloudflare R2 into local disk.
#
# Counterpart of sync_v86_to_r2.sh. Run this once BEFORE switching the backend
# to STORAGE_BACKEND=fs so games already published from R2 keep working: keys
# are content-addressed on both sides (v86/assets/systems/{sha256}/...,
# v86/games/{sha256}/..., v86/snapshots/..., v86/saves/...), so the sync is
# idempotent and can be re-run safely.
#
# After switching to fs mode and verifying a game boots, the bucket copy can
# be deleted to reclaim R2 storage (deliberately manual):
#   aws s3 rm --endpoint-url "$R2_ENDPOINT" s3://$R2_BUCKET/v86/ --recursive
#
# Usage:
#   cd blog/backend
#   ./sync_r2_to_fs.sh             # uses .env values + ./project-demos
#   ./sync_r2_to_fs.sh <dir>       # override the project-demos root
#   DRY_RUN=1 ./sync_r2_to_fs.sh  # show what would be downloaded
#
# Requirements:
#   - aws CLI installed (https://docs.aws.amazon.com/cli/latest/userguide/)
#   - R2_ACCOUNT_ID / R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_BUCKET
#     in ./.env (or exported) -- use the credentials of the environment you
#     are migrating (local .env = dev bucket, VPS .env = prod bucket)
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DEMOS="${1:-$BASE_DIR/project-demos}"

# ── Load R2 config from backend/.env if present ────────────────────────────
if [[ -f "$BASE_DIR/.env" ]]; then
	set -a
	# shellcheck disable=SC1091
	source "$BASE_DIR/.env"
	set +a
fi

R2_ACCOUNT_ID="${R2_ACCOUNT_ID:?R2_ACCOUNT_ID is not set (put it in ./.env or export it)}"
R2_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID:?R2_ACCESS_KEY_ID is not set}"
R2_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY:?R2_SECRET_ACCESS_KEY is not set}"
R2_BUCKET="${R2_BUCKET:?R2_BUCKET is not set}"
R2_ENDPOINT="${R2_ENDPOINT:-https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com}"

export AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="$R2_SECRET_ACCESS_KEY" AWS_DEFAULT_REGION=auto

SYNC_ARGS=()
[[ "${DRY_RUN:-0}" == "1" ]] && SYNC_ARGS+=(--dryrun)

mkdir -p "$PROJECT_DEMOS/v86"

echo "Syncing s3://$R2_BUCKET/v86/ -> $PROJECT_DEMOS/v86/"
aws s3 sync "s3://$R2_BUCKET/v86/" "$PROJECT_DEMOS/v86/" \
	--endpoint-url "$R2_ENDPOINT" \
	--exclude "v86/tmp/*" \
	"${SYNC_ARGS[@]}" \
	--only-show-errors

echo
echo "Done. Verify with:"
echo "  find $PROJECT_DEMOS/v86 -type f | wc -l"
echo "  aws s3 ls --endpoint-url $R2_ENDPOINT s3://$R2_BUCKET/v86/ --recursive | wc -l"
