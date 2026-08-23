#!/usr/bin/env bash
#
# Backfill v86 assets from local disk into Cloudflare R2.
#
# The upload pipeline writes new system/game chunks straight to R2, so this
# script is only needed when local disk holds legacy chunks that predate the
# pipeline (or a chunk went missing). Keys are content-addressed on both sides
# (v86/assets/systems/{sha256}/{start}-{end}.img.zst), so the sync is
# idempotent: identical bytes map to identical keys and are skipped.
#
# Usage:
#   cd blog/backend
#   ./sync_v86_to_r2.sh             # uses .env values + ./project-demos
#   ./sync_v86_to_r2.sh <dir>       # override the project-demos root
#   DRY_RUN=1 ./sync_v86_to_r2.sh  # show what would be uploaded
#
# Requirements:
#   - aws CLI installed (https://docs.aws.amazon.com/cli/latest/userguide/)
#   - R2_ACCOUNT_ID / R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_BUCKET
#     in ./.env (or exported) -- use the credentials of the environment you
#     want to fix (local .env = dev bucket, VPS .env = prod bucket)
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

SYNC_ARGS=(--cache-control "public, max-age=31536000, immutable" --content-type "application/octet-stream")
[[ "${DRY_RUN:-0}" == "1" ]] && SYNC_ARGS+=(--dryrun)

synced=0
for dir in systems games; do
	src="$PROJECT_DEMOS/v86/assets/$dir"
	[[ -d "$src" ]] || continue
	echo "Syncing $src -> s3://$R2_BUCKET/v86/assets/$dir/"
	aws s3 sync "$src/" "s3://$R2_BUCKET/v86/assets/$dir/" \
		--endpoint-url "$R2_ENDPOINT" \
		"${SYNC_ARGS[@]}" \
		--only-show-errors
	synced=$((synced + 1))
done

[[ $synced -gt 0 ]] || { echo "nothing to sync: no v86 assets found under $PROJECT_DEMOS" >&2; exit 1; }

echo
echo "Done. Verify with:"
echo "  aws s3 ls --endpoint-url $R2_ENDPOINT s3://$R2_BUCKET/v86/assets/systems/ --recursive | awk '{print \$3}' | sort | uniq -c | head"
