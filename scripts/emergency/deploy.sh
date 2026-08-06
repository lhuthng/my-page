#!/usr/bin/env bash
#
# Emergency deploy: build image on this machine -> push to GHCR -> pull & reload on the VPS.
#
# This is a TEMPORARY replacement for the GitHub Actions pipeline
# (`.github/workflows/deploy.yml`) and is intended for manual use only
# while that automation is unavailable or broken.
#
# Repo layout expected (run from anywhere; the script locates the repo root):
#   <root>/blog/<service>/   Docker build context for frontend / backend
#
# Usage:
#   scripts/emergency/deploy.sh --check
#   scripts/emergency/deploy.sh [frontend|backend|all]    # default: frontend
#
# One-time prerequisites (NOT automated, done by the human doing the deploy):
#   * Start the local Docker daemon (e.g. Docker Desktop / OrbStack / colima).
#   * `docker login ghcr.io` with a token that has `write:packages`.
#   * Have SSH access to the VPS with a key that lets you log in.
#   * Copy deploy.env.example to deploy.env (gitignored) and fill it in.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="$SCRIPT_DIR/deploy.env"

# ---- Load config (environment overrides take precedence) ----
load_config() {
	VM_HOST="${VM_HOST:-}"
	VM_USER="${VM_USER:-}"
	VM_SSH_KEY="${VM_SSH_KEY:-}"
	VM_DEPLOY_PATH="${VM_DEPLOY_PATH:-~/MyPage/blog}"
	GHCR_IMAGE="${GHCR_IMAGE:-ghcr.io/$(id -un)}"

	if [[ -f "$ENV_FILE" ]]; then
		# shellcheck disable=SC1090
		source "$ENV_FILE"
	fi
}

log()  { printf '[deploy] %s\n' "$*"; }
die()  { printf '[deploy] ERROR: %s\n' "$*" >&2; exit 1; }

# ---- Pre-flight checks ----
ghcr_can_push() {
	python3 - "$GHCR_IMAGE" <<'PY'
import sys, json, subprocess, base64, urllib.request, urllib.error
image = sys.argv[1]
try:
    cfg = json.load(open('/Users/' + __import__('os').getlogin() + '/.docker/config.json'))
except Exception:
    cfg = {}
store = cfg.get('credsStore') or cfg.get('credsHelpers', {}).get('ghcr.io')
if not store:
    sys.exit(1)
raw = subprocess.run(['docker-credential-' + store, 'get'], input='ghcr.io',
                     capture_output=True, text=True).stdout
j = json.loads(raw)
token = j.get('Secret') or j.get('secret')
if not token:
    sys.exit(1)
b64 = base64.b64encode(((j.get('Username') or '') + ':' + token).encode()).decode()
url = ('https://ghcr.io/token?service=ghcr.io&scope=repository:%s/blog-frontend:pull,push' % image)
req = urllib.request.Request(url, headers={'Authorization': 'Basic ' + b64})
try:
    with urllib.request.urlopen(req) as r:
        access = json.load(r).get('access', [])
    sys.exit(0 if any('blog-frontend' in g.get('name', '') and 'push' in g.get('actions', []) for g in access) else 1)
except Exception:
    sys.exit(1)
PY
}

check_prereqs() {
	local ok=1

	log "Checking prerequisites (config: $ENV_FILE)"
	[[ -n "$VM_HOST" ]] || { echo "  [ ] VM_HOST  (set in $ENV_FILE or env)"; ok=0; }
	[[ -n "$VM_USER" ]] || { echo "  [ ] VM_USER  (set in $ENV_FILE or env)"; ok=0; }
	[[ -n "$VM_SSH_KEY" ]] || { echo "  [ ] VM_SSH_KEY  (path to your VPS SSH key)"; ok=0; }

	if docker info >/dev/null 2>&1; then
		log "  [x] Docker daemon is running"
	else
		log "  [ ] Docker daemon is NOT running (start Docker Desktop/OrbStack/colima)"; ok=0
	fi

	if docker buildx version >/dev/null 2>&1; then
		log "  [x] docker buildx available"
	else
		log "  [ ] docker buildx missing"; ok=0
	fi

	if ghcr_can_push; then
		log "  [x] ghcr.io credential has push scope for ${GHCR_IMAGE}/blog-frontend"
	else
		log "  [ ] ghcr.io token lacks push scope (create a CLASSIC PAT with write:packages, then: docker login ghcr.io)"; ok=0
	fi

	if [[ -n "$VM_HOST" && -n "$VM_USER" ]]; then
		if ssh -i "${VM_SSH_KEY:-/dev/null}" -o IdentitiesOnly=yes -o ConnectTimeout=8 \
			-o BatchMode=yes "${VM_USER}@${VM_HOST}" 'echo ok' >/dev/null 2>&1; then
			log "  [x] SSH to ${VM_USER}@${VM_HOST} works"
		else
			log "  [ ] SSH to ${VM_USER}@${VM_HOST} FAILED"; ok=0
		fi
	fi

	[[ $ok -eq 1 ]] && log "All prerequisites satisfied." || { die "Fix the flagged items above."; }
}

# ---- Build + push an image, then reload it on the VPS ----
deploy_service() {
	local service="$1"
	local image="${GHCR_IMAGE}/blog-${service}:latest"
	log "==> Building and pushing ${image} (from blog/${service})"
	cd "$REPO_ROOT"
	docker buildx build --platform linux/amd64 --push -t "$image" "blog/${service}"

	log "==> Pulling and reloading ${service} on ${VM_USER}@${VM_HOST}"
	ssh -i "$VM_SSH_KEY" -o IdentitiesOnly=yes "${VM_USER}@${VM_HOST}" \
		"cd ${VM_DEPLOY_PATH} && docker compose pull ${service} && docker compose up -d ${service} && docker image prune -f"
	log "==> ${service} deployed."
}

# ---- Smoke-test that the site responds ----
verify() {
	log "==> Verifying live site"
	local urls=("https://blog.huuthangle.site/robots.txt" "https://blog.huuthangle.site/BingSiteAuth.xml")
	for url in "${urls[@]}"; do
		local code
		code="$(curl -sS -o /dev/null -w '%{http_code}' --connect-timeout 10 "$url" 2>/dev/null || echo 000)"
		printf "  %-55s -> %s\n" "$url" "$code"
	done
	log "If /BingSiteAuth.xml shows 404 while others show 200, purge that URL in Cloudflare."
}

main() {
	load_config
	local mode="${1:-frontend}"

	case "$mode" in
		--check|-c)
			check_prereqs
			;;
		frontend|backend)
			check_prereqs_no_ssh_ping
			deploy_service "$mode"
			verify
			;;
		all)
			check_prereqs_no_ssh_ping
			deploy_service "backend"
			deploy_service "frontend"
			verify
			;;
		*)
			die "usage: $0 [frontend|backend|all|--check]"
			;;
	esac
}

# Full prereq check bails on SSH failure; for an actual deploy we want to
# proceed even if we can't confirm SSH (it will fail the ssh step anyway).
check_prereqs_no_ssh_ping() {
	if [[ -z "$VM_HOST" || -z "$VM_USER" || -z "$VM_SSH_KEY" ]]; then
		die "Missing VM config. Copy scripts/emergency/deploy.env.example to scripts/emergency/deploy.env and fill it in."
	fi
	if ! docker info >/dev/null 2>&1; then
		die "Docker daemon is not running."
	fi
	if ! docker buildx version >/dev/null 2>&1; then
		die "docker buildx is not available."
	fi
}

main "$@"