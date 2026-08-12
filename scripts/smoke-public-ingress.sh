#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-${1:-}}"
EXPECT_HOME_TEXT="${EXPECT_HOME_TEXT:-我和狗蛋儿的家 · 主城群聊}"
EXPECT_RESIDENT_TEXT="${EXPECT_RESIDENT_TEXT:-我和狗蛋儿的家 · 住宅}"
EXPECT_ADMIN_TEXT="${EXPECT_ADMIN_TEXT:-AJW聊天 · 管理后台}"
EXPECT_PROVIDER_FRAGMENT="${EXPECT_PROVIDER_FRAGMENT:-\"reachable\":true}"
EXPECT_CORS_ORIGIN="${EXPECT_CORS_ORIGIN:-}"
EXPECT_RELEASE_GIT_SHA="${EXPECT_RELEASE_GIT_SHA:-}"
EXPECT_MANIFEST_CONTENT_TYPE="${EXPECT_MANIFEST_CONTENT_TYPE:-application/json}"
CURL_BIN="${CURL_BIN:-curl}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

require_non_empty() {
  local name="$1"
  local value="$2"
  if [[ -z "$value" ]]; then
    echo "missing required value: $name" >&2
    exit 1
  fi
}

mktemp_file() {
  local file
  file="$(mktemp "${TMPDIR:-/tmp}/lobster-public-smoke.XXXXXX" 2>/dev/null)" \
    || file="$(mktemp -t lobster-public-smoke)"
  printf '%s\n' "$file"
}

fetch_body() {
  local url="$1"
  local output="$2"
  "$CURL_BIN" -fsS "$url" -o "$output"
}

fetch_body_with_headers() {
  local url="$1"
  local output="$2"
  local headers="$3"
  "$CURL_BIN" -fsS -D "$headers" "$url" -o "$output"
}

fetch_head_status() {
  local url="$1"
  "$CURL_BIN" -fsSI "$url" | head -n 1
}

extract_git_sha() {
  local input="$1"
  tr -d '\r\n' < "$input" \
    | sed -nE 's/.*"git_sha"[[:space:]]*:[[:space:]]*"([0-9a-fA-F]{40})".*/\1/p'
}

fetch_status() {
  local method="$1"
  local url="$2"
  "$CURL_BIN" -sS -X "$method" -o /dev/null -w '%{http_code}' "$url"
}

assert_status() {
  local expected="$1"
  local method="$2"
  local url="$3"
  local actual
  actual="$(fetch_status "$method" "$url")"
  [[ "$actual" == "$expected" ]] || {
    echo "unexpected HTTP status for $url: expected $expected, got $actual" >&2
    exit 1
  }
}

need_cmd "$CURL_BIN"
need_cmd grep
need_cmd head
need_cmd mktemp
need_cmd sed
need_cmd tr

require_non_empty "BASE_URL" "$BASE_URL"
BASE_URL="${BASE_URL%/}"

BODY_FILE="$(mktemp_file)"
HEADER_FILE="$(mktemp_file)"
trap 'rm -f "$BODY_FILE" "$HEADER_FILE"' EXIT

echo "== public ingress smoke =="
echo "base: $BASE_URL"

echo "== homepage =="
fetch_body "$BASE_URL/" "$BODY_FILE"
grep -F "$EXPECT_HOME_TEXT" "$BODY_FILE" >/dev/null || {
  echo "homepage did not contain expected marker: $EXPECT_HOME_TEXT" >&2
  exit 1
}

echo "== H5 resident page =="
fetch_body "$BASE_URL/creative.html" "$BODY_FILE"
grep -F "$EXPECT_RESIDENT_TEXT" "$BODY_FILE" >/dev/null || {
  echo "resident page did not contain expected marker: $EXPECT_RESIDENT_TEXT" >&2
  exit 1
}

echo "== admin page =="
fetch_body "$BASE_URL/admin-ds.html" "$BODY_FILE"
grep -F "$EXPECT_ADMIN_TEXT" "$BODY_FILE" >/dev/null || {
  echo "admin page did not contain expected marker: $EXPECT_ADMIN_TEXT" >&2
  exit 1
}

echo "== GET /health =="
fetch_body "$BASE_URL/health" "$BODY_FILE"
if [[ "$(cat "$BODY_FILE")" != "ok" ]]; then
  echo "unexpected /health body:" >&2
  cat "$BODY_FILE" >&2
  exit 1
fi

echo "== HEAD /health =="
health_status="$(fetch_head_status "$BASE_URL/health")"
printf '%s\n' "$health_status"
printf '%s' "$health_status" | grep -F "200" >/dev/null || {
  echo "HEAD /health did not return 200" >&2
  exit 1
}

echo "== /v1/provider =="
fetch_body "$BASE_URL/v1/provider" "$BODY_FILE"
grep -F "$EXPECT_PROVIDER_FRAGMENT" "$BODY_FILE" >/dev/null || {
  echo "provider response missing expected fragment: $EXPECT_PROVIDER_FRAGMENT" >&2
  cat "$BODY_FILE" >&2
  exit 1
}

echo "== /v1/version =="
fetch_body "$BASE_URL/v1/version" "$BODY_FILE"
version_git_sha="$(extract_git_sha "$BODY_FILE")"
[[ "$version_git_sha" =~ ^[0-9a-fA-F]{40}$ ]] || {
  echo "/v1/version did not expose a valid 40-character git_sha" >&2
  cat "$BODY_FILE" >&2
  exit 1
}
printf 'runtime git_sha: %s\n' "$version_git_sha"

echo "== /release-manifest.json =="
fetch_body_with_headers "$BASE_URL/release-manifest.json" "$BODY_FILE" "$HEADER_FILE"
grep -Fi "content-type: $EXPECT_MANIFEST_CONTENT_TYPE" "$HEADER_FILE" >/dev/null || {
  echo "release manifest did not return expected content type: $EXPECT_MANIFEST_CONTENT_TYPE" >&2
  grep -Fi "content-type:" "$HEADER_FILE" >&2 || true
  exit 1
}
manifest_git_sha="$(extract_git_sha "$BODY_FILE")"
[[ "$manifest_git_sha" == "$version_git_sha" ]] || {
  echo "runtime git_sha does not match release manifest git_sha" >&2
  printf 'runtime: %s\nmanifest: %s\n' "$version_git_sha" "$manifest_git_sha" >&2
  exit 1
}
if [[ -n "$EXPECT_RELEASE_GIT_SHA" ]]; then
  [[ "$EXPECT_RELEASE_GIT_SHA" =~ ^[0-9a-fA-F]{40}$ ]] || {
    echo "EXPECT_RELEASE_GIT_SHA must be a 40-character hexadecimal Git SHA" >&2
    exit 1
  }
  [[ "$version_git_sha" == "$EXPECT_RELEASE_GIT_SHA" ]] || {
    echo "deployed git_sha does not match EXPECT_RELEASE_GIT_SHA" >&2
    printf 'expected: %s\nactual: %s\n' "$EXPECT_RELEASE_GIT_SHA" "$version_git_sha" >&2
    exit 1
  }
fi

echo "== protected route without bearer =="
assert_status "401" "GET" "$BASE_URL/v1/admin/summary"
assert_status "401" "POST" "$BASE_URL/v1/auth/logout"

echo "== anonymous shell state privacy =="
fetch_body "$BASE_URL/v1/shell/state" "$BODY_FILE"
if grep -Eq '"id"[[:space:]]*:[[:space:]]*"dm:' "$BODY_FILE"; then
  echo "anonymous shell state exposed a direct conversation" >&2
  exit 1
fi

if [[ -n "$EXPECT_CORS_ORIGIN" ]]; then
  echo "== CORS origin =="
  headers="$($CURL_BIN -fsS -D - -o /dev/null -H "Origin: ${EXPECT_CORS_ORIGIN}" "$BASE_URL/health")"
  printf '%s\n' "$headers" | grep -Fi "Access-Control-Allow-Origin: ${EXPECT_CORS_ORIGIN}" >/dev/null || {
    echo "public CORS origin did not match: $EXPECT_CORS_ORIGIN" >&2
    exit 1
  }
fi

echo "public ingress smoke passed"
