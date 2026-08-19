#!/usr/bin/env bash
#
# One-time migration of v86 assets (Windows 9x base systems + game ISOs) to
# Cloudflare R2 so the browser can fetch them straight from the CDN instead
# of being served in 256KB chunks by the backend.
#
# R2 keys are content-addressed and mirror the URLs the frontend will use:
#   systems -> v86/assets/systems/{version_id}/{sha256}/{start}-{end}.img.zst
#   games   -> v86/games/{iso_sha256}/full.iso
#
# Usage:
#   cd blog/backend
#   ./migrate_to_r2.sh          # uses .env values + this repo's project-demos
#   ./migrate_to_r2.sh <dir>    # override the project-demos root
#
# Requirements:
#   - aws CLI installed (https://docs.aws.amazon.com/cli/latest/userguide/)
#   - R2_ACCOUNT_ID / R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_BUCKET
#     in ./backend/.env (or exported)
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DEMOS="${1:-$BASE_DIR/project-demos}"
STAGING="${STAGING_DIR:-$(mktemp -d /tmp/r2-migrate.XXXXXX)}"
trap 'rm -rf "$STAGING"' EXIT

# ── Load R2 config from backend/.env if present ────────────────────────────
if [[ -f "$BASE_DIR/.env" ]]; then
	set -a
	# shellcheck disable=SC1091
	source "$BASE_DIR/.env"
	set +a
fi

R2_ACCOUNT_ID="${R2_ACCOUNT_ID:?R2_ACCOUNT_ID is not set (put it in backend/.env or export it)}"
R2_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID:?R2_ACCESS_KEY_ID is not set}"
R2_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY:?R2_SECRET_ACCESS_KEY is not set}"
R2_BUCKET="${R2_BUCKET:?R2_BUCKET is not set}"
R2_ENDPOINT="${R2_ENDPOINT:-https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com}"

export AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="$R2_SECRET_ACCESS_KEY" AWS_DEFAULT_REGION=auto

[[ -d "$PROJECT_DEMOS/v86/systems" ]] || [[ -d "$PROJECT_DEMOS/v86/games" ]] || {
	echo "error: no v86 data found under $PROJECT_DEMOS/v86" >&2
	exit 1
}

mkdir -p "$STAGING/v86/assets/systems" "$STAGING/v86/games"

# ── Stage system parts → v86/assets/systems/{version_id}/{sha}/{part} ───────
system_count=0
while IFS= read -r -d '' part; do
	# part = .../v86/systems/{system_id}/{version_id}/{sha}/parts/{file}
	sha="$(basename "$(dirname "$(dirname "$part")")")"
	version_id="$(basename "$(dirname "$(dirname "$(dirname "$part")")")")"
	key="$STAGING/v86/assets/systems/$version_id/$sha/$(basename "$part")"
	mkdir -p "$(dirname "$key")"
	ln -f "$part" "$key" 2>/dev/null || cp "$part" "$key"
	system_count=$((system_count + 1))
done < <(find "$PROJECT_DEMOS/v86/systems" -name '*.img.zst' -print0)

# ── Stage game ISOs → v86/games/{iso_sha}/full.iso ──────────────────────────
game_count=0
while IFS= read -r -d '' iso; do
	# iso = .../v86/games/{uuid}/{sha}/game.iso|full.iso
	sha="$(basename "$(dirname "$iso")")"
	key="$STAGING/v86/games/$sha/full.iso"
	mkdir -p "$(dirname "$key")"
	ln -f "$iso" "$key" 2>/dev/null || cp "$iso" "$key"
	game_count=$((game_count + 1))
done < <(find "$PROJECT_DEMOS/v86/games" \( -name 'game.iso' -o -name 'full.iso' \) -print0)

# ── Stage game ZIPs → v86/games/zips/{zip_sha}.zip ──────────────────────────
# The reuse-source upload flow re-builds the ISO from the stored ZIP, so the
# originals must live in R2 too (content-addressed by their sha256 name).
zip_count=0
while IFS= read -r -d '' zip; do
	# zip = .../v86/games/{uuid}/{zip_sha}.zip
	name="$(basename "$zip")"
	key="$STAGING/v86/games/zips/$name"
	mkdir -p "$(dirname "$key")"
	ln -f "$zip" "$key" 2>/dev/null || cp "$zip" "$key"
	zip_count=$((zip_count + 1))
done < <(find "$PROJECT_DEMOS/v86/games" -name '*.zip' -print0)

echo "Staged ${system_count} system parts + ${game_count} game ISOs + ${zip_count} game ZIPs under $STAGING"

# ── Upload (aws s3 sync is resumable + idempotent) ─────────────────────────
aws s3 sync "$STAGING/" "s3://$R2_BUCKET/" \
	--endpoint-url "$R2_ENDPOINT" \
	--cache-control "public, max-age=31536000, immutable" \
	--content-type "application/octet-stream" \
	--only-show-errors

echo
echo "Migration complete."
echo "Verify with:"
echo "  aws s3 ls --endpoint-url $R2_ENDPOINT s3://$R2_BUCKET/v86/assets/systems/ --recursive | head"
echo "  aws s3 ls --endpoint-url $R2_ENDPOINT s3://$R2_BUCKET/v86/games/ --recursive | head"
echo
echo "Public URLs (via $R2_PUBLIC_URL):"
echo "  https://$R2_BUCKET.example/v86/games/{iso_sha}/full.iso"
