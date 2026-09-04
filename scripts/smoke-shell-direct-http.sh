#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-}"
KEEP_STATE="${KEEP_STATE:-0}"
SKIP_BUILD="${SKIP_BUILD:-0}"
GATEWAY_BIN="${GATEWAY_BIN:-$ROOT_DIR/target/debug/lobster-waku-gateway}"
GATEWAY_PID=""
EVENTS_PID=""

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
  dir="$(mktemp -d "${TMPDIR:-/tmp}/lobster-shell-direct-http.XXXXXX" 2>/dev/null)" \
    || dir="$(mktemp -d -t lobster-shell-direct-http)"
  printf '%s\n' "$dir"
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

json_assert() {
  local payload="$1"
  local mode="$2"
  JSON_PAYLOAD="$payload" python3 - "$mode" <<'PY'
import json
import os
import sys

mode = sys.argv[1]
raw_payload = os.environ["JSON_PAYLOAD"]
payload = None if mode in {"event-body", "edited-event-body", "recalled-event-body"} else json.loads(raw_payload)

ROOM_ID = "dm:qa-a:qa-b"
SMOKE_TEXT = os.environ.get("SMOKE_TEXT", "")
SMOKE_EDIT_TEXT = os.environ.get("SMOKE_EDIT_TEXT", "")

def shell_rooms(doc):
    if isinstance(doc.get("rooms"), list):
        return doc["rooms"]
    shell = doc.get("conversation_shell") or {}
    return shell.get("conversations") or []

def find_room(doc, room_id=ROOM_ID):
    for room in shell_rooms(doc):
        if room.get("id") == room_id or room.get("conversation_id") == room_id:
            return room
    return None

def room_messages(room):
    return room.get("messages") or []

def find_message(doc, text=SMOKE_TEXT):
    for room in shell_rooms(doc):
        for message in room_messages(room):
            if message.get("text") == text:
                return room, message
    return None, None

if mode == "direct-open":
    assert payload["conversation_id"] == ROOM_ID
    assert payload["group_id"] == "mls:dm:qa-a:qa-b"
    assert payload["kind"] == "Direct"
    assert payload["scope"] == "Private"
    members = {
        member.get("identity_id"): member.get("device_id")
        for member in payload.get("members", [])
    }
    assert members.get("qa-a") == "browser-a"
    assert members.get("qa-b") == "browser-b"
elif mode == "initial-state":
    assert isinstance(payload.get("state_version"), str) and payload["state_version"]
    room = find_room(payload)
    assert room is not None, "participant direct room must exist before direct message"
    assert room.get("kind") == "direct"
    assert room.get("scope") == "private"
    assert room.get("self_label") == "qa-b"
    assert room.get("peer_label") == "qa-a"
elif mode == "send-response":
    assert payload["delivery_status"] == "delivered"
    assert payload["message_id"]
    assert payload["sender"] == "qa-a"
    assert payload["text"] == SMOKE_TEXT
elif mode == "edit-response":
    assert payload["ok"] is True
    assert payload["message_id"] == os.environ["MESSAGE_ID"]
    assert payload["edit_status"] == "edited"
    assert payload["edited_by"] == "qa-a"
    assert payload["text"] == SMOKE_EDIT_TEXT
elif mode == "recall-response":
    assert payload["ok"] is True
    assert payload["message_id"] == os.environ["MESSAGE_ID"]
    assert payload["recall_status"] == "recalled"
    assert payload["recalled_by"] == "qa-a"
elif mode == "peer-state":
    room, message = find_message(payload)
    assert room is not None, "peer viewer must see the direct message"
    assert room.get("id") == ROOM_ID or room.get("conversation_id") == ROOM_ID
    assert room.get("kind") == "direct"
    assert room.get("scope") == "private"
    assert room.get("self_label") == "qa-b"
    assert room.get("peer_label") == "qa-a"
    assert message["sender"] == "qa-a"
    assert message["delivery_status"] == "delivered"
elif mode == "edited-peer-state":
    room, message = find_message(payload, SMOKE_EDIT_TEXT)
    assert room is not None, "peer viewer must see the edited direct message"
    assert message["message_id"] == os.environ["MESSAGE_ID"]
    assert message["sender"] == "qa-a"
    assert message["delivery_status"] == "delivered"
    assert message["is_edited"] is True
    assert message["edited_by"] == "qa-a"
elif mode == "recalled-peer-state":
    room = find_room(payload)
    assert room is not None, "peer viewer must retain the recalled direct message"
    recalled = None
    for message in room_messages(room):
        if message.get("message_id") == os.environ["MESSAGE_ID"]:
            recalled = message
            break
    assert recalled is not None, "recalled message id missing"
    assert recalled["sender"] == "qa-a"
    assert recalled["delivery_status"] == "delivered"
    assert recalled["text"] == "消息已撤回"
    assert recalled["is_recalled"] is True
    assert recalled["recalled_by"] == "qa-a"
elif mode == "outsider-state":
    assert find_room(payload) is None, "outsider must not see the direct room"
    assert SMOKE_TEXT not in raw_payload, "outsider must not see direct message text"
    assert SMOKE_EDIT_TEXT not in raw_payload, "outsider must not see edited direct text"
elif mode == "blocked-response":
    message = payload["Error"]["message"]
    assert "not a participant" in message
elif mode == "event-body":
    assert "event: shell-state" in raw_payload
    assert SMOKE_TEXT in raw_payload
    data_lines = [
        line.removeprefix("data: ")
        for line in raw_payload.splitlines()
        if line.startswith("data: ") and SMOKE_TEXT in line
    ]
    assert data_lines, "SSE shell-state must include the direct smoke message"
    event_payload = json.loads(data_lines[-1])
    event_room, event_message = find_message(event_payload)
    assert event_room is not None, "SSE direct room projection missing"
    assert event_room.get("id") == ROOM_ID or event_room.get("conversation_id") == ROOM_ID
    assert event_message["sender"] == "qa-a"
    assert event_message["delivery_status"] == "delivered"
elif mode == "edited-event-body":
    assert "event: shell-state" in raw_payload
    assert SMOKE_EDIT_TEXT in raw_payload
    data_lines = [
        line.removeprefix("data: ")
        for line in raw_payload.splitlines()
        if line.startswith("data: ") and SMOKE_EDIT_TEXT in line
    ]
    assert data_lines, "SSE shell-state must include the edited direct message"
    event_payload = json.loads(data_lines[-1])
    event_room, event_message = find_message(event_payload, SMOKE_EDIT_TEXT)
    assert event_room is not None, "SSE edited direct room projection missing"
    assert event_message["message_id"] == os.environ["MESSAGE_ID"]
    assert event_message["delivery_status"] == "delivered"
    assert event_message["is_edited"] is True
    assert event_message["edited_by"] == "qa-a"
elif mode == "recalled-event-body":
    assert "event: shell-state" in raw_payload
    assert "消息已撤回" in raw_payload
    data_lines = [
        line.removeprefix("data: ")
        for line in raw_payload.splitlines()
        if line.startswith("data: ") and os.environ["MESSAGE_ID"] in line
    ]
    assert data_lines, "SSE shell-state must include the recalled direct message id"
    event_payload = json.loads(data_lines[-1])
    event_room = find_room(event_payload)
    assert event_room is not None, "SSE recalled direct room projection missing"
    recalled = None
    for message in room_messages(event_room):
        if message.get("message_id") == os.environ["MESSAGE_ID"]:
            recalled = message
            break
    assert recalled is not None, "SSE recalled direct message missing"
    assert recalled["text"] == "消息已撤回"
    assert recalled["delivery_status"] == "delivered"
    assert recalled["is_recalled"] is True
    assert recalled["recalled_by"] == "qa-a"
else:
    raise AssertionError(f"unsupported mode: {mode}")
PY
}

state_version_from_payload() {
  local payload="$1"
  JSON_PAYLOAD="$payload" python3 - <<'PY'
import json
import os
print(json.loads(os.environ["JSON_PAYLOAD"])["state_version"])
PY
}

urlencode_value() {
  local value="$1"
  RAW_VALUE="$value" python3 - <<'PY'
import os
import urllib.parse
print(urllib.parse.quote(os.environ["RAW_VALUE"], safe=""))
PY
}

start_peer_events_after_state() {
  local state_payload="$1"
  local version
  local encoded
  version="$(state_version_from_payload "$state_payload")"
  encoded="$(urlencode_value "$version")"
  curl -fsS \
    "$GATEWAY_URL/v1/shell/events?resident_id=qa-b&after=$encoded&wait_ms=4000" \
    >"$EVENTS_BODY" &
  EVENTS_PID="$!"
  sleep 0.2
}

wait_peer_events_assert() {
  local mode="$1"
  wait "$EVENTS_PID"
  EVENTS_PID=""
  json_assert "$(cat "$EVENTS_BODY")" "$mode"
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
  echo "== building lobster-waku-gateway =="
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway >/dev/null
fi

if [[ ! -x "$GATEWAY_BIN" ]]; then
  echo "gateway binary not found: $GATEWAY_BIN" >&2
  exit 1
fi

if [[ -z "$PORT" ]]; then
  PORT="$(reserve_port)"
fi

STATE_ROOT="$(mktemp_dir)"
GATEWAY_LOG="$STATE_ROOT/gateway.log"
EVENTS_BODY="$STATE_ROOT/events.body"
SMOKE_TEXT="${SMOKE_TEXT:-SHELL_DIRECT_HTTP_SMOKE_$(date +%s)}"
SMOKE_EDIT_TEXT="${SMOKE_EDIT_TEXT:-SHELL_DIRECT_HTTP_EDIT_$(date +%s)}"
export SMOKE_TEXT
export SMOKE_EDIT_TEXT

cleanup() {
  local exit_code=$?
  if [[ -n "${EVENTS_PID:-}" ]] && kill -0 "$EVENTS_PID" >/dev/null 2>&1; then
    kill "$EVENTS_PID" >/dev/null 2>&1 || true
    wait "$EVENTS_PID" >/dev/null 2>&1 || true
  fi
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

echo "== starting gateway on :$PORT =="
# This fixture uses synthetic qa-* identities; keep the development bypass explicit.
LOBSTER_DEV_AUTH_BYPASS=1 "$GATEWAY_BIN" \
  --host "$HOST" \
  --port "$PORT" \
  --state-dir "$STATE_ROOT/gateway" \
  >"$GATEWAY_LOG" 2>&1 &
GATEWAY_PID="$!"
wait_for_health "http://$HOST:$PORT/health"
GATEWAY_URL="http://$HOST:$PORT"

echo "== opening qa-a to qa-b direct room =="
direct_file="$STATE_ROOT/direct-open.json"
curl -fsS \
  -X POST "$GATEWAY_URL/v1/direct/open" \
  -H 'content-type: application/json' \
  -d '{"requester_id":"qa-a","requester_device_id":"browser-a","peer_id":"qa-b","peer_device_id":"browser-b"}' \
  >"$direct_file"
json_assert "$(cat "$direct_file")" "direct-open"

echo "== reading qa-b initial direct shell state =="
initial_state="$(curl -fsS "$GATEWAY_URL/v1/shell/state?resident_id=qa-b")"
json_assert "$initial_state" "initial-state"

echo "== waiting for qa-b direct shell events =="
start_peer_events_after_state "$initial_state"

echo "== qa-a sends direct shell message =="
send_response_file="$STATE_ROOT/send-response.json"
curl -fsS \
  -X POST "$GATEWAY_URL/v1/shell/message" \
  -H 'content-type: application/json' \
  -d "{\"room_id\":\"dm:qa-a:qa-b\",\"sender\":\"qa-a\",\"text\":\"$SMOKE_TEXT\",\"device_id\":\"shell-direct-http-smoke\",\"language_tag\":\"zh-CN\"}" \
  >"$send_response_file"
send_response="$(cat "$send_response_file")"
json_assert "$send_response" "send-response"
MESSAGE_ID="$(
  JSON_PAYLOAD="$send_response" python3 - <<'PY'
import json
import os
print(json.loads(os.environ["JSON_PAYLOAD"])["message_id"])
PY
)"
export MESSAGE_ID

wait_peer_events_assert "event-body"

echo "== qa-b sees qa-a delivered direct message =="
peer_state="$(curl -fsS "$GATEWAY_URL/v1/shell/state?resident_id=qa-b")"
json_assert "$peer_state" "peer-state"

echo "== waiting for qa-b direct edit events =="
start_peer_events_after_state "$peer_state"

echo "== qa-a edits direct shell message =="
edit_response_file="$STATE_ROOT/edit-response.json"
curl -fsS \
  -X POST "$GATEWAY_URL/v1/shell/message/edit" \
  -H 'content-type: application/json' \
  -d "{\"room_id\":\"dm:qa-a:qa-b\",\"message_id\":\"$MESSAGE_ID\",\"actor\":\"qa-a\",\"text\":\"$SMOKE_EDIT_TEXT\"}" \
  >"$edit_response_file"
json_assert "$(cat "$edit_response_file")" "edit-response"
wait_peer_events_assert "edited-event-body"

echo "== qa-b sees edited direct message =="
edited_peer_state="$(curl -fsS "$GATEWAY_URL/v1/shell/state?resident_id=qa-b")"
json_assert "$edited_peer_state" "edited-peer-state"

echo "== waiting for qa-b direct recall events =="
start_peer_events_after_state "$edited_peer_state"

echo "== qa-a recalls direct shell message =="
recall_response_file="$STATE_ROOT/recall-response.json"
curl -fsS \
  -X POST "$GATEWAY_URL/v1/shell/message/recall" \
  -H 'content-type: application/json' \
  -d "{\"room_id\":\"dm:qa-a:qa-b\",\"message_id\":\"$MESSAGE_ID\",\"actor\":\"qa-a\"}" \
  >"$recall_response_file"
json_assert "$(cat "$recall_response_file")" "recall-response"
wait_peer_events_assert "recalled-event-body"

echo "== qa-b sees recalled direct message =="
recalled_peer_state="$(curl -fsS "$GATEWAY_URL/v1/shell/state?resident_id=qa-b")"
json_assert "$recalled_peer_state" "recalled-peer-state"

echo "== qa-c cannot see qa-a to qa-b direct room =="
outsider_state="$(curl -fsS "$GATEWAY_URL/v1/shell/state?resident_id=qa-c")"
json_assert "$outsider_state" "outsider-state"

echo "== qa-c cannot write into qa-a to qa-b direct room =="
blocked_file="$STATE_ROOT/blocked-response.json"
blocked_code="$(
  curl -sS \
    -o "$blocked_file" \
    -w '%{http_code}' \
    -X POST "$GATEWAY_URL/v1/shell/message" \
    -H 'content-type: application/json' \
    -d '{"room_id":"dm:qa-a:qa-b","sender":"qa-c","text":"outsider write should be rejected","device_id":"browser-c","language_tag":"zh-CN"}'
)"
if [[ "$blocked_code" != "400" ]]; then
  echo "expected outsider write to fail with 400, got $blocked_code" >&2
  cat "$blocked_file" >&2
  exit 1
fi
json_assert "$(cat "$blocked_file")" "blocked-response"

echo "== webpush endpoints (direct) =="
vapid_file="$STATE_ROOT/vapid-key.json"
curl -fsS "$GATEWAY_URL/v1/push/vapid-public-key" >"$vapid_file"
json_assert "$(cat "$vapid_file")" "vapid-key"
grep -q '"public_key"' "$vapid_file" || {
  echo "vapid key payload missing public_key" >&2
  exit 1
}
# dev bypass 下订阅写操作仍必须拒绝匿名调用（401）
push_code="$(
  curl -sS -o "$STATE_ROOT/push-401.json" -w '%{http_code}' \
    -X POST "$GATEWAY_URL/v1/push/subscribe" \
    -H 'content-type: application/json' \
    -d '{"endpoint":"https://push.example/abc","keys":{"p256dh":"K","auth":"A"}}'
)"
if [[ "$push_code" != "401" ]]; then
  echo "expected anonymous push subscribe to fail with 401, got $push_code" >&2
  cat "$STATE_ROOT/push-401.json" >&2
  exit 1
fi
json_assert "$(cat "$STATE_ROOT/push-401.json")" "push-401-response"

echo "== shell direct HTTP smoke passed =="
echo "gateway: $GATEWAY_URL"
echo "message: $SMOKE_TEXT"
