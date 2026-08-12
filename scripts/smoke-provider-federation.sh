#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${BIN_PATH:-$ROOT_DIR/target/release/lobster-waku-gateway}"
GATEWAY_ARTIFACT="${GATEWAY_ARTIFACT:-}"
HOST="${HOST:-127.0.0.1}"
UPSTREAM_PORT="${UPSTREAM_PORT:-}"
DOWNSTREAM_PORT="${DOWNSTREAM_PORT:-}"
KEEP_STATE="${KEEP_STATE:-0}"
SKIP_BUILD="${SKIP_BUILD:-0}"
FEDERATION_TOKEN="${FEDERATION_TOKEN:-lobster-federation-smoke-token}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

mktemp_dir() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/lobster-chat-smoke.XXXXXX" 2>/dev/null)" \
    || dir="$(mktemp -d -t lobster-chat-smoke)"
  printf '%s\n' "$dir"
}

wait_for_health() {
  local name="$1"
  local url="$2"
  local attempt
  for attempt in $(seq 1 60); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  echo "timed out waiting for ${name} health: ${url}" >&2
  return 1
}

need_cmd curl
need_cmd grep
need_cmd mktemp
need_cmd tar
need_cmd python3

# Both gateways are local processes; never route their health/state probes through
# a user-configured HTTP proxy when this smoke is run directly.
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"
export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"

reserve_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

if [[ -n "$GATEWAY_ARTIFACT" ]]; then
  if [[ ! -f "$GATEWAY_ARTIFACT" ]]; then
    echo "gateway artifact not found: $GATEWAY_ARTIFACT" >&2
    exit 1
  fi
fi

if [[ "$SKIP_BUILD" != "1" && -z "$GATEWAY_ARTIFACT" ]]; then
  need_cmd cargo
  echo "== building lobster-waku-gateway =="
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway
fi

EXTRACT_DIR=""

if [[ -n "$GATEWAY_ARTIFACT" ]]; then
  EXTRACT_DIR="$(mktemp_dir)"
  tar -xzf "$GATEWAY_ARTIFACT" -C "$EXTRACT_DIR"
  BIN_PATH="$EXTRACT_DIR/lobster-waku-gateway"
fi

if [[ ! -x "$BIN_PATH" ]]; then
  echo "gateway binary not found: $BIN_PATH" >&2
  if [[ -n "$EXTRACT_DIR" && "$KEEP_STATE" != "1" && -d "$EXTRACT_DIR" ]]; then
    rm -rf "$EXTRACT_DIR"
  fi
  exit 1
fi

if [[ -z "$UPSTREAM_PORT" ]]; then
  UPSTREAM_PORT="$(reserve_port)"
fi
if [[ -z "$DOWNSTREAM_PORT" ]]; then
  while :; do
    DOWNSTREAM_PORT="$(reserve_port)"
    [[ "$DOWNSTREAM_PORT" != "$UPSTREAM_PORT" ]] && break
  done
fi
if [[ "$UPSTREAM_PORT" == "$DOWNSTREAM_PORT" ]]; then
  echo "upstream and downstream ports must differ: $UPSTREAM_PORT" >&2
  exit 1
fi

STATE_ROOT="$(mktemp_dir)"
UPSTREAM_LOG="$STATE_ROOT/upstream.log"
DOWNSTREAM_LOG="$STATE_ROOT/downstream.log"
UPSTREAM_PID=""
DOWNSTREAM_PID=""

cleanup() {
  local exit_code=$?
  if [[ -n "$DOWNSTREAM_PID" ]] && kill -0 "$DOWNSTREAM_PID" >/dev/null 2>&1; then
    kill "$DOWNSTREAM_PID" >/dev/null 2>&1 || true
    wait "$DOWNSTREAM_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$UPSTREAM_PID" ]] && kill -0 "$UPSTREAM_PID" >/dev/null 2>&1; then
    kill "$UPSTREAM_PID" >/dev/null 2>&1 || true
    wait "$UPSTREAM_PID" >/dev/null 2>&1 || true
  fi
  if [[ "$KEEP_STATE" != "1" && -d "$STATE_ROOT" ]]; then
    rm -rf "$STATE_ROOT"
  fi
  if [[ -n "$EXTRACT_DIR" && "$KEEP_STATE" != "1" && -d "$EXTRACT_DIR" ]]; then
    rm -rf "$EXTRACT_DIR"
  fi
  exit "$exit_code"
}
trap cleanup EXIT

echo "== starting upstream gateway on :$UPSTREAM_PORT =="
# This fixture uses synthetic smoke-bot identities, so keep the resident auth
# bypass explicit. Federation itself still uses a dedicated inbound token.
LOBSTER_DEV_AUTH_BYPASS=1 \
LOBSTER_GATEWAY_FEDERATION_TOKEN="$FEDERATION_TOKEN" \
"$BIN_PATH" \
  --host "$HOST" \
  --port "$UPSTREAM_PORT" \
  --state-dir "$STATE_ROOT/upstream" \
  >"$UPSTREAM_LOG" 2>&1 &
UPSTREAM_PID="$!"

# Downstream startup performs an eager upstream health check. Wait for the
# upstream listener first so a normal cold start cannot be misclassified as a
# federation failure.
wait_for_health "upstream" "http://$HOST:$UPSTREAM_PORT/health"

echo "== starting downstream gateway on :$DOWNSTREAM_PORT bridged to upstream =="
LOBSTER_DEV_AUTH_BYPASS=1 \
LOBSTER_WAKU_UPSTREAM_TOKEN="$FEDERATION_TOKEN" \
"$BIN_PATH" \
  --host "$HOST" \
  --port "$DOWNSTREAM_PORT" \
  --state-dir "$STATE_ROOT/downstream" \
  --upstream-gateway-url "http://$HOST:$UPSTREAM_PORT" \
  >"$DOWNSTREAM_LOG" 2>&1 &
DOWNSTREAM_PID="$!"

if ! wait_for_health "downstream" "http://$HOST:$DOWNSTREAM_PORT/health"; then
  echo "downstream log: $DOWNSTREAM_LOG" >&2
  tail -n 80 "$DOWNSTREAM_LOG" >&2 || true
  exit 1
fi

provider_json="$(curl -fsS "http://$HOST:$DOWNSTREAM_PORT/v1/provider")"
printf '%s' "$provider_json" | grep -q '"mode":"remote-gateway"' || {
  echo "downstream gateway did not report remote-gateway mode" >&2
  echo "$provider_json" >&2
  exit 1
}
printf '%s' "$provider_json" | grep -q '"reachable":true' || {
  echo "downstream gateway reports upstream as unreachable" >&2
  echo "$provider_json" >&2
  exit 1
}

message="smoke-provider-federation-$(date +%s)"
payload="$(cat <<EOF
{"room_id":"room:world:lobby","sender":"smoke-bot","text":"$message","device_id":"smoke-script","language_tag":"en"}
EOF
)"

echo "== publishing shell message through downstream =="
curl -fsS \
  -H 'Content-Type: application/json' \
  -d "$payload" \
  "http://$HOST:$DOWNSTREAM_PORT/v1/shell/message" \
  >/dev/null

found=0
for _ in $(seq 1 60); do
  shell_state="$(curl -fsS "http://$HOST:$UPSTREAM_PORT/v1/shell/state" || true)"
  if printf '%s' "$shell_state" | grep -Fq "$message"; then
    found=1
    break
  fi
  sleep 0.25
done

if [[ "$found" != "1" ]]; then
  echo "upstream shell state never observed the downstream message" >&2
  echo "upstream log:   $UPSTREAM_LOG" >&2
  echo "downstream log: $DOWNSTREAM_LOG" >&2
  exit 1
fi

echo "== provider federation smoke passed =="
if [[ -n "$GATEWAY_ARTIFACT" ]]; then
  echo "gateway artifact: $GATEWAY_ARTIFACT"
else
  echo "gateway binary: $BIN_PATH"
fi
echo "provider status: $provider_json"
echo "state root: $STATE_ROOT"
if [[ "$KEEP_STATE" != "1" ]]; then
  echo "logs were temporary; rerun with KEEP_STATE=1 to inspect them."
fi
