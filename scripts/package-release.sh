#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist}"
SKIP_BUILD="${SKIP_BUILD:-0}"
HOST_TARGET_OVERRIDE="${HOST_TARGET_OVERRIDE:-}"
GATEWAY_BINARY_PATH="${GATEWAY_BINARY_PATH:-$ROOT_DIR/target/release/lobster-waku-gateway}"
ALLOW_DIRTY_RELEASE="${ALLOW_DIRTY_RELEASE:-0}"
RELEASE_GIT_SHA="${RELEASE_GIT_SHA:-}"
RELEASE_BUILT_AT="${RELEASE_BUILT_AT:-}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

need_cmd tar
need_cmd git

if [[ "$ALLOW_DIRTY_RELEASE" != "1" && -n "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=normal)" ]]; then
  echo "refusing release package from dirty worktree; commit/stash changes or set ALLOW_DIRTY_RELEASE=1 for an explicit local-only build" >&2
  exit 1
fi

if [[ -z "$RELEASE_GIT_SHA" ]]; then
  RELEASE_GIT_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD)"
fi
if [[ ! "$RELEASE_GIT_SHA" =~ ^[0-9a-fA-F]{40}$ ]]; then
  echo "invalid RELEASE_GIT_SHA: expected a full 40-character commit id" >&2
  exit 1
fi
if [[ -z "$RELEASE_BUILT_AT" ]]; then
  RELEASE_BUILT_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
fi

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  else
    need_cmd shasum
    shasum -a 256 "$1" | awk '{ print $1 }'
  fi
}

mkdir -p "$DIST_DIR"

if [[ -z "$HOST_TARGET_OVERRIDE" ]]; then
  need_cmd rustc
  host_target="$(rustc -vV | awk '/host:/ { print $2 }')"
else
  host_target="$HOST_TARGET_OVERRIDE"
fi
bin_name="lobster-waku-gateway-${host_target}"
binary_path="$GATEWAY_BINARY_PATH"

if [[ "$SKIP_BUILD" != "1" ]]; then
  need_cmd cargo
  echo "== building release gateway for $host_target =="
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway
fi

echo "== packaging source archive =="
tar \
  --exclude="$(basename "$ROOT_DIR")/.git" \
  --exclude="$(basename "$ROOT_DIR")/.playwright-cli" \
  --exclude=".DS_Store" \
  --exclude="*/.DS_Store" \
  --exclude="$(basename "$ROOT_DIR")/node_modules" \
  --exclude="*/node_modules" \
  --exclude="$(basename "$ROOT_DIR")/target" \
  --exclude="$(basename "$ROOT_DIR")/dist" \
  --exclude="$(basename "$ROOT_DIR")/.lobster-chat-dev" \
  --exclude="$(basename "$ROOT_DIR")/backups" \
  --exclude="*/backups" \
  --exclude="*/test-results" \
  --exclude="*/screenshots" \
  --exclude="*/.tmp" \
  --exclude="*.source.html" \
  --exclude="*/*.source.html" \
  -czf "$DIST_DIR/lobster-chat-source.tar.gz" \
  -C "$(dirname "$ROOT_DIR")" \
  "$(basename "$ROOT_DIR")"

echo "== packaging H5 shell =="
tar \
  --exclude="./node_modules" \
  --exclude="./backups" \
  --exclude="./.tmp" \
  --exclude="./test" \
  --exclude="./test-results" \
  --exclude="./screenshots" \
  --exclude="./*.mjs" \
  --exclude="./.DS_Store" \
  --exclude="*/.DS_Store" \
  --exclude="*.source.html" \
  --exclude="*/*.source.html" \
  -czf "$DIST_DIR/lobster-web-shell.tar.gz" \
  -C "$ROOT_DIR/apps/lobster-web-shell" \
  .

if [[ -x "$binary_path" ]]; then
  echo "== packaging gateway binary for $host_target =="
  tar -czf "$DIST_DIR/${bin_name}.tar.gz" -C "$(dirname "$binary_path")" lobster-waku-gateway
else
  echo "warning: release gateway binary not found at $binary_path" >&2
fi

source_sha="$(sha256_file "$DIST_DIR/lobster-chat-source.tar.gz")"
web_sha="$(sha256_file "$DIST_DIR/lobster-web-shell.tar.gz")"
gateway_file="${bin_name}.tar.gz"
gateway_sha=""
gateway_json="null"
if [[ -f "$DIST_DIR/$gateway_file" ]]; then
  gateway_sha="$(sha256_file "$DIST_DIR/$gateway_file")"
  gateway_json="{\"file\":\"$gateway_file\",\"sha256\":\"$gateway_sha\"}"
fi

printf '%s\n' \
  "{\"schema_version\":1,\"git_sha\":\"$RELEASE_GIT_SHA\",\"built_at\":\"$RELEASE_BUILT_AT\",\"target\":\"$host_target\",\"artifacts\":{\"source\":{\"file\":\"lobster-chat-source.tar.gz\",\"sha256\":\"$source_sha\"},\"web\":{\"file\":\"lobster-web-shell.tar.gz\",\"sha256\":\"$web_sha\"},\"gateway\":$gateway_json}}" \
  > "$DIST_DIR/release-manifest.json"

echo "artifacts written to $DIST_DIR"
