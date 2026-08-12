#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${ENV_FILE:-/etc/lobster-chat/gateway.env}"
BASE_URL="${BASE_URL:-}"
CHECK_PUBLIC="${CHECK_PUBLIC:-0}"
EXPECT_RELEASE_GIT_SHA="${EXPECT_RELEASE_GIT_SHA:-}"

fail() {
  echo "production readiness failed: $*" >&2
  exit 1
}

[[ -r "$ENV_FILE" ]] || fail "environment file is not readable: $ENV_FILE"
[[ "$CHECK_PUBLIC" == "0" || "$CHECK_PUBLIC" == "1" ]] || fail "CHECK_PUBLIC must be 0 or 1"

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

cors_origin="${LOBSTER_CORS_ORIGIN:-}"
[[ "$cors_origin" =~ ^https://[^[:space:]]+$ ]] || fail "LOBSTER_CORS_ORIGIN must be a single https origin"
[[ "$cors_origin" != "*" ]] || fail "LOBSTER_CORS_ORIGIN must not be *"
[[ "${LOBSTER_DEV_AUTH_BYPASS:-0}" == "0" ]] || fail "LOBSTER_DEV_AUTH_BYPASS must be 0 or unset"
[[ "${LOBSTER_DEV_EMAIL_OTP_INLINE:-0}" == "0" ]] || fail "LOBSTER_DEV_EMAIL_OTP_INLINE must be 0 or unset"
secure_session_master_key="${LOBSTER_SECURE_SESSION_MASTER_KEY:-}"
[[ "${#secure_session_master_key}" -ge 32 ]] \
  || fail "LOBSTER_SECURE_SESSION_MASTER_KEY must be at least 32 characters"

mailer_url="${LOBSTER_EMAIL_OTP_MAILER_URL:-}"
if [[ ! "$mailer_url" =~ ^https://[^[:space:]]+$ ]]; then
  # 与 Gateway email_otp_mailer 的例外一致:仅放行 loopback http(同机 lobster-mailer)
  [[ "$mailer_url" =~ ^http://(127\.0\.0\.1|localhost|\[::1\])[:/][^[:space:]]*$ ]] \
    || fail "LOBSTER_EMAIL_OTP_MAILER_URL must use https (loopback http allowed for co-located mailer)"
fi
[[ -n "${LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN:-}" ]] || fail "LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN is empty"

upstream_url="${LOBSTER_WAKU_UPSTREAM_URL:-}"
if [[ -n "$upstream_url" ]]; then
  if [[ ! "$upstream_url" =~ ^https://[^[:space:]]+$ ]]; then
    [[ "$upstream_url" =~ ^http://(127\.0\.0\.1|localhost|\[::1\])[:/][^[:space:]]*$ ]] \
      || fail "LOBSTER_WAKU_UPSTREAM_URL must use https (loopback http allowed)"
  fi
  [[ -n "${LOBSTER_WAKU_UPSTREAM_TOKEN:-}" ]] \
    || fail "LOBSTER_WAKU_UPSTREAM_TOKEN is required when an upstream gateway is configured"
fi

if [[ "$CHECK_PUBLIC" == "1" ]]; then
  [[ "$BASE_URL" =~ ^https://[^/]+/?$ ]] || fail "BASE_URL must be an https origin"
  base="${BASE_URL%/}"
  curl -fsS "$base/health" >/dev/null || fail "public health probe failed"
  curl -fsS "$base/v1/provider" >/dev/null || fail "public provider probe failed"
  headers="$(curl -fsS -D - -o /dev/null -H "Origin: ${LOBSTER_CORS_ORIGIN}" "$base/health")" || fail "public CORS probe failed"
  printf '%s\n' "$headers" | grep -Fqi "Access-Control-Allow-Origin: ${LOBSTER_CORS_ORIGIN}" || fail "public CORS origin does not match configured origin"
  manifest_file="$(mktemp "${TMPDIR:-/tmp}/lobster-release-manifest.XXXXXX")" || fail "cannot create release manifest temp file"
  manifest_headers="$(mktemp "${TMPDIR:-/tmp}/lobster-release-headers.XXXXXX")" || fail "cannot create release header temp file"
  trap 'rm -f "$manifest_file" "$manifest_headers"' EXIT
  version_json="$(curl -fsS "$base/v1/version")" || fail "public version probe failed"
  version_git_sha="$(printf '%s' "$version_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["git_sha"])')" \
    || fail "public /v1/version did not contain git_sha"
  [[ "$version_git_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail "public /v1/version git_sha is invalid"
  curl -fsS -D "$manifest_headers" -o "$manifest_file" "$base/release-manifest.json" \
    || fail "public release manifest probe failed"
  grep -Fqi "content-type: application/json" "$manifest_headers" \
    || fail "public release manifest is not served as application/json"
  manifest_git_sha="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["git_sha"])' "$manifest_file")" \
    || fail "public release manifest did not contain git_sha"
  [[ "$manifest_git_sha" == "$version_git_sha" ]] \
    || fail "public runtime git_sha does not match release manifest git_sha"
  if [[ -n "$EXPECT_RELEASE_GIT_SHA" ]]; then
    [[ "$EXPECT_RELEASE_GIT_SHA" =~ ^[0-9a-fA-F]{40}$ ]] \
      || fail "EXPECT_RELEASE_GIT_SHA must be a 40-character hexadecimal Git SHA"
    [[ "$version_git_sha" == "$EXPECT_RELEASE_GIT_SHA" ]] \
      || fail "public runtime git_sha does not match EXPECT_RELEASE_GIT_SHA"
  fi
  echo "production config and public probes/version traceability passed"
else
  echo "production config readiness passed (public checks skipped)"
fi
