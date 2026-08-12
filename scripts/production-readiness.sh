#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${ENV_FILE:-/etc/lobster-chat/gateway.env}"
BASE_URL="${BASE_URL:-}"
CHECK_PUBLIC="${CHECK_PUBLIC:-0}"

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
  echo "production config and public probes passed"
else
  echo "production config readiness passed (public checks skipped)"
fi
