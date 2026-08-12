#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-}"
KEEP_STATE="${KEEP_STATE:-0}"
SKIP_BUILD="${SKIP_BUILD:-0}"
GATEWAY_BIN="${GATEWAY_BIN:-$ROOT_DIR/target/debug/lobster-waku-gateway}"
CLI_BIN="${CLI_BIN:-$ROOT_DIR/target/debug/lobster-cli}"
TUI_BIN="${TUI_BIN:-$ROOT_DIR/target/debug/lobster-tui}"
GATEWAY_PID=""

# This smoke starts a local Gateway; keep health and API probes off user proxies.
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"
export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

mktemp_dir() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/lobster-resident-smoke.XXXXXX" 2>/dev/null)" || dir="$(mktemp -d -t lobster-resident-smoke)"
  printf '%s
' "$dir"
}

wait_for_health() {
  local url="$1"
  local attempt
  for attempt in $(seq 1 80); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  echo "timed out waiting for gateway health: $url" >&2
  return 1
}

need_cmd curl
need_cmd python3

reserve_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

if [[ "$SKIP_BUILD" != "1" ]]; then
  need_cmd cargo
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway -p lobster-cli -p lobster-tui >/dev/null
fi

if [[ ! -x "$GATEWAY_BIN" ]]; then
  echo "gateway binary not found: $GATEWAY_BIN" >&2
  exit 1
fi

if [[ ! -x "$CLI_BIN" ]]; then
  echo "cli binary not found: $CLI_BIN" >&2
  exit 1
fi

if [[ ! -x "$TUI_BIN" ]]; then
  echo "tui binary not found: $TUI_BIN" >&2
  exit 1
fi

if [[ -z "$PORT" ]]; then
  PORT="$(reserve_port)"
fi

STATE_ROOT="$(mktemp_dir)"
export LOBSTER_WEB_GENERATED_DIR="$STATE_ROOT/web-generated"
GATEWAY_LOG="$STATE_ROOT/gateway.log"
GATEWAY_URL="http://$HOST:$PORT"
RESIDENT_ID="novel-reader"
JOIN_TEXT="USER_RESIDENT_MAINLINE_SMOKE_首条消息"
DM_PEER_ID="builder"
DM_TEXT="USER_RESIDENT_DM_SMOKE_私帖首条消息"
DM_CONVERSATION_ID="dm:builder:novel-reader"
RESIDENCE_CONVERSATION_ID="dm:guide:novel-reader"

cleanup() {
  local exit_code=$?
  if [[ -n "$GATEWAY_PID" ]] && kill -0 "$GATEWAY_PID" >/dev/null 2>&1; then
    kill "$GATEWAY_PID" >/dev/null 2>&1 || true
    wait "$GATEWAY_PID" >/dev/null 2>&1 || true
  fi
  if [[ "$KEEP_STATE" != "1" && -d "$STATE_ROOT" ]]; then
    rm -rf "$STATE_ROOT"
  fi
  exit "$exit_code"
}
trap cleanup EXIT

# This fixture probes an unregistered synthetic identity; keep the dev bypass explicit.
LOBSTER_DEV_EMAIL_OTP_INLINE=1 LOBSTER_DEV_AUTH_BYPASS=1 "$GATEWAY_BIN"   --host "$HOST"   --port "$PORT"   --state-dir "$STATE_ROOT/gateway"   >"$GATEWAY_LOG" 2>&1 &
GATEWAY_PID="$!"
wait_for_health "$GATEWAY_URL/health"

preflight_allowed="$(curl -fsS -X POST "$GATEWAY_URL/v1/auth/preflight" -H 'content-type: application/json' -d '{"email":"novel.reader@example.com","mobile":"+86 13800138000","device_physical_address":"66:55:44:33:22:11"}')"
JSON_PAYLOAD="$preflight_allowed" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert payload['allowed'] is True
assert payload['blocked_reasons'] == []
PY2

otp_request="$(curl -fsS -X POST "$GATEWAY_URL/v1/auth/email-otp/request" -H 'content-type: application/json' -d '{"email":"novel.reader@example.com","mobile":"+86 13800138000","device_physical_address":"66:55:44:33:22:11","resident_id":"novel-reader"}')"
challenge_id="$(JSON_PAYLOAD="$otp_request" python3 - <<'PY2'
import json, os
print(json.loads(os.environ['JSON_PAYLOAD'])['challenge_id'])
PY2
)"
dev_code="$(JSON_PAYLOAD="$otp_request" python3 - <<'PY2'
import json, os
print(json.loads(os.environ['JSON_PAYLOAD'])['dev_code'])
PY2
)"

otp_verify="$(curl -fsS -X POST "$GATEWAY_URL/v1/auth/email-otp/verify" -H 'content-type: application/json' -d "{\"challenge_id\":\"$challenge_id\",\"code\":\"$dev_code\",\"resident_id\":\"$RESIDENT_ID\"}")"
JSON_PAYLOAD="$otp_verify" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert payload['resident_id'] == 'novel-reader'
assert payload['state'] == 'Active'
PY2
session_token="$(JSON_PAYLOAD="$otp_verify" python3 - <<'PY2'
import json, os
print(json.loads(os.environ['JSON_PAYLOAD'])['session_token'])
PY2
)"

join_unregistered_body="$STATE_ROOT/join-unregistered.json"
join_unregistered_status="$(curl -sS -o "$join_unregistered_body" -w '%{http_code}' -X POST "$GATEWAY_URL/v1/cities/join" -H 'content-type: application/json' -d '{"city":"core-harbor","resident_id":"guest-01"}')"
[[ "$join_unregistered_status" == "400" ]] || {
  echo "expected unregistered join to fail with 400" >&2
  cat "$join_unregistered_body" >&2 || true
  exit 1
}
grep -F 'not registered' "$join_unregistered_body" >/dev/null || {
  echo "expected unregistered join response to mention registration" >&2
  cat "$join_unregistered_body" >&2 || true
  exit 1
}

join_registered="$(curl -fsS -X POST "$GATEWAY_URL/v1/cities/join" -H 'content-type: application/json' -H "authorization: Bearer $session_token" -d '{"city":"core-harbor","resident_id":"novel-reader"}')"
JSON_PAYLOAD="$join_registered" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert payload['resident_id'] == 'novel-reader'
assert payload['state'] == 'Active'
PY2

bootstrap_message="TUI_GATEWAY_BOOTSTRAP_SMOKE_来自正式 shell state"
curl -fsS \
  -X POST "$GATEWAY_URL/v1/shell/message" \
  -H 'content-type: application/json' \
  -H "authorization: Bearer $session_token" \
  -d "{\"room_id\":\"$RESIDENCE_CONVERSATION_ID\",\"sender\":\"$RESIDENT_ID\",\"text\":\"$bootstrap_message\"}" \
  >"$STATE_ROOT/bootstrap-message.json"

residents_json="$(curl -fsS "$GATEWAY_URL/v1/residents")"
JSON_PAYLOAD="$residents_json" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
record = next(item for item in payload if item['resident_id'] == 'novel-reader')
assert 'core-harbor' in record['active_cities']
PY2

rooms_json="$($CLI_BIN rooms --for "user:$RESIDENT_ID" --token "$session_token" --gateway "$GATEWAY_URL" --json)"
JSON_PAYLOAD="$rooms_json" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
conversation_ids = {entry['conversation_id'] for entry in payload['entries']}
assert 'room:city:core-harbor:lobby' in conversation_ids
assert 'dm:guide:novel-reader' in conversation_ids
PY2

direct_snapshot="$(
  LOBSTER_SESSION_TOKEN="$session_token" \
  LOBSTER_WAKU_GATEWAY_URL="$GATEWAY_URL" \
  LOBSTER_TUI_STATE_DIR="$STATE_ROOT/tui-direct" \
  LOBSTER_TUI_RESIDENT_ID="$RESIDENT_ID" \
  LOBSTER_TUI_SMOKE_DUMP=json \
  "$TUI_BIN" --mode direct
)"
JSON_PAYLOAD="$direct_snapshot" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert payload['surface_kind'] == 'ResidenceDirect'
assert payload['active_conversation_id'] == 'dm:guide:novel-reader'
assert payload['message_count'] >= 1
assert 'TUI_GATEWAY_BOOTSTRAP_SMOKE' in payload['latest_message_hint']
assert 'actions' in payload['visible_panels']
PY2

tui_script_output="$(
  LOBSTER_SESSION_TOKEN="$session_token" \
  LOBSTER_WAKU_GATEWAY_URL="$GATEWAY_URL" \
  LOBSTER_TUI_STATE_DIR="$STATE_ROOT/tui-state" \
  LOBSTER_TUI_RESIDENT_ID="$RESIDENT_ID" \
  LOBSTER_TUI_SMOKE_DUMP=plain \
  LOBSTER_TUI_SMOKE_SCRIPT="$(printf '%s\n%s\n%s\n%s\n' "$JOIN_TEXT" "/dm $DM_PEER_ID" "$DM_TEXT" "/search $DM_TEXT")" \
  "$TUI_BIN" --mode user
)"
grep -F "搜索「${DM_TEXT}」命中" <<<"$tui_script_output" >/dev/null || {
  echo "TUI scoped search smoke did not report a hit" >&2
  printf '%s\n' "$tui_script_output" >&2
  exit 1
}

tail_json="$($CLI_BIN tail --for "user:$RESIDENT_ID" --conversation-id room:city:core-harbor:lobby --token "$session_token" --gateway "$GATEWAY_URL" --json)"
JSON_PAYLOAD="$tail_json" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert any(item['text'] == 'USER_RESIDENT_MAINLINE_SMOKE_首条消息' for item in payload['messages'])
PY2

dm_tail_json="$($CLI_BIN tail --for "user:$RESIDENT_ID" --conversation-id "$DM_CONVERSATION_ID" --token "$session_token" --gateway "$GATEWAY_URL" --json)"
JSON_PAYLOAD="$dm_tail_json" python3 - <<'PY2'
import json, os
payload = json.loads(os.environ['JSON_PAYLOAD'])
assert any(item['text'] == 'USER_RESIDENT_DM_SMOKE_私帖首条消息' for item in payload['messages'])
PY2

echo "resident mainline smoke passed"
echo "gateway: $GATEWAY_URL"
echo "state root: $STATE_ROOT"
