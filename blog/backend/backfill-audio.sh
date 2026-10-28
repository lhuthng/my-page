#!/usr/bin/env bash
# Run without flags first; --prune <fingerprint> applies that exact R2-only plan.
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$BASE_DIR/../docker-compose.yml"

if ! command -v docker >/dev/null 2>&1; then
	printf 'backfill-audio: docker is not installed\n' >&2
	exit 1
fi
if [[ ! -f "$COMPOSE_FILE" ]]; then
	printf 'backfill-audio: no compose file at %s\n' "$COMPOSE_FILE" >&2
	exit 1
fi

exec docker compose -f "$COMPOSE_FILE" exec -T backend backfill-audio "$@"
