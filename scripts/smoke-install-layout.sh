#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KEEP_STATE="${KEEP_STATE:-0}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

mktemp_dir() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/lobster-install-smoke.XXXXXX" 2>/dev/null)" \
    || dir="$(mktemp -d -t lobster-install-smoke)"
  printf '%s\n' "$dir"
}

assert_contains() {
  local file="$1"
  local text="$2"
  grep -F "$text" "$file" >/dev/null || {
    echo "expected '$text' in $file" >&2
    exit 1
  }
}

need_cmd bash
need_cmd rustc
need_cmd tar

STATE_ROOT="$(mktemp_dir)"
FAKE_BIN="$STATE_ROOT/fake-bin"
ARTIFACT_ROOT="$STATE_ROOT/artifacts"
INSTALL_ROOT="$STATE_ROOT/install-root"
STATE_DIR="$STATE_ROOT/state"
ETC_ROOT="$STATE_ROOT/etc"
LOG_FILE="$STATE_ROOT/commands.log"
INVALID_NAME_LOG="$STATE_ROOT/invalid-server-name.log"
SERVICE_NAME="lobster-waku-gateway-smoke"
HOST_TARGET="$(rustc -vV | awk '/host:/ { print $2 }')"
GATEWAY_ARTIFACT="$ARTIFACT_ROOT/lobster-waku-gateway-${HOST_TARGET}.tar.gz"
WEB_ARTIFACT="$ARTIFACT_ROOT/lobster-web-shell.tar.gz"
RELEASE_MANIFEST="$ARTIFACT_ROOT/release-manifest.json"
SMOKE_GIT_SHA="1111111111111111111111111111111111111111"
SYSTEMD_UNIT="$ETC_ROOT/systemd/${SERVICE_NAME}.service"
NGINX_SITE_DEBIAN="$ETC_ROOT/nginx/sites-available/lobster-chat"
NGINX_LINK_DEBIAN="$ETC_ROOT/nginx/sites-enabled/lobster-chat"
NGINX_DEFAULT_SITE_DEBIAN="$ETC_ROOT/nginx/sites-enabled/default"
NGINX_SITE_RHEL="$ETC_ROOT/nginx/conf.d/lobster-chat.conf"
NGINX_SERVER_NAME="chat.example.com"

cleanup() {
  local exit_code=$?
  if [[ "$KEEP_STATE" != "1" && -d "$STATE_ROOT" ]]; then
    rm -rf "$STATE_ROOT"
  fi
  exit "$exit_code"
}
trap cleanup EXIT

mkdir -p \
  "$FAKE_BIN" \
  "$ARTIFACT_ROOT" \
  "$(dirname "$SYSTEMD_UNIT")" \
  "$(dirname "$NGINX_SITE_DEBIAN")" \
  "$(dirname "$NGINX_LINK_DEBIAN")"

cat >"$FAKE_BIN/systemctl" <<'EOF'
#!/usr/bin/env bash
echo "systemctl $*" >>"$SMOKE_LOG"
if [[ "${1:-}" == "is-active" && "${3:-}" == "nginx" ]]; then
  exit 1
fi
exit 0
EOF

cat >"$FAKE_BIN/pgrep" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF

cat >"$FAKE_BIN/nginx" <<'EOF'
#!/usr/bin/env bash
echo "nginx $*" >>"$SMOKE_LOG"
exit 0
EOF

cat >"$FAKE_BIN/curl" <<'EOF'
#!/usr/bin/env bash
url="${*: -1}"
echo "curl $url" >>"$SMOKE_LOG"
case "$url" in
  */health)
    printf '{"ok":true,"status":"live"}'
    ;;
  */v1/version)
    printf '{"schema_version":1,"package_version":"smoke","git_sha":"%s"}' "$SMOKE_GIT_SHA"
    ;;
  */v1/provider)
    printf '{"provider":"mock","reachable":true}'
    ;;
  */release-manifest.json)
    cat "$RELEASE_MANIFEST"
    ;;
  *)
    echo "unexpected curl url: $url" >&2
    exit 1
    ;;
esac
EOF

chmod +x "$FAKE_BIN/systemctl" "$FAKE_BIN/nginx" "$FAKE_BIN/curl" "$FAKE_BIN/pgrep"

mkdir -p "$STATE_ROOT/fake-gateway"
cat >"$STATE_ROOT/fake-gateway/lobster-waku-gateway" <<'EOF'
#!/usr/bin/env bash
echo "fake gateway"
EOF
chmod +x "$STATE_ROOT/fake-gateway/lobster-waku-gateway"
tar -czf "$GATEWAY_ARTIFACT" -C "$STATE_ROOT/fake-gateway" lobster-waku-gateway

mkdir -p "$STATE_ROOT/fake-web"
cat >"$STATE_ROOT/fake-web/index.html" <<'EOF'
<!doctype html>
<html><body>lobster-web-shell smoke</body></html>
EOF
tar -czf "$WEB_ARTIFACT" -C "$STATE_ROOT/fake-web" .

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  else
    shasum -a 256 "$1" | awk '{ print $1 }'
  fi
}

printf '%s\n' \
  "{\"schema_version\":1,\"git_sha\":\"$SMOKE_GIT_SHA\",\"built_at\":\"2026-08-12T00:00:00Z\",\"target\":\"$HOST_TARGET\",\"artifacts\":{\"source\":null,\"web\":{\"file\":\"$(basename "$WEB_ARTIFACT")\",\"sha256\":\"$(sha256_file "$WEB_ARTIFACT")\"},\"gateway\":{\"file\":\"$(basename "$GATEWAY_ARTIFACT")\",\"sha256\":\"$(sha256_file "$GATEWAY_ARTIFACT")\"}}}" \
  >"$RELEASE_MANIFEST"

echo "== install layout smoke =="
PATH="$FAKE_BIN:$PATH" \
SMOKE_LOG="$LOG_FILE" \
INSTALL_ROOT="$INSTALL_ROOT" \
STATE_DIR="$STATE_DIR" \
SERVICE_NAME="$SERVICE_NAME" \
GATEWAY_ARTIFACT="$GATEWAY_ARTIFACT" \
WEB_ARTIFACT="$WEB_ARTIFACT" \
RELEASE_MANIFEST="$RELEASE_MANIFEST" \
SMOKE_GIT_SHA="$SMOKE_GIT_SHA" \
SYSTEMD_UNIT="$SYSTEMD_UNIT" \
NGINX_SITE_DEBIAN="$NGINX_SITE_DEBIAN" \
NGINX_LINK_DEBIAN="$NGINX_LINK_DEBIAN" \
NGINX_DEFAULT_SITE_DEBIAN="$NGINX_DEFAULT_SITE_DEBIAN" \
NGINX_SITE_RHEL="$NGINX_SITE_RHEL" \
NGINX_SERVER_NAME="$NGINX_SERVER_NAME" \
PUBLIC_PORT=8080 \
bash "$ROOT_DIR/scripts/install-server.sh"

if PATH="$FAKE_BIN:$PATH" \
  SMOKE_LOG="$LOG_FILE" \
  INSTALL_ROOT="$INSTALL_ROOT" \
  STATE_DIR="$STATE_DIR" \
  SERVICE_NAME="$SERVICE_NAME" \
  GATEWAY_ARTIFACT="$GATEWAY_ARTIFACT" \
  WEB_ARTIFACT="$WEB_ARTIFACT" \
  RELEASE_MANIFEST="$RELEASE_MANIFEST" \
  SMOKE_GIT_SHA="$SMOKE_GIT_SHA" \
  SYSTEMD_UNIT="$SYSTEMD_UNIT" \
  NGINX_SITE_DEBIAN="$NGINX_SITE_DEBIAN" \
  NGINX_LINK_DEBIAN="$NGINX_LINK_DEBIAN" \
  NGINX_DEFAULT_SITE_DEBIAN="$NGINX_DEFAULT_SITE_DEBIAN" \
  NGINX_SITE_RHEL="$NGINX_SITE_RHEL" \
  NGINX_SERVER_NAME='chat.example.com; include /tmp/unsafe.conf' \
  PUBLIC_PORT=8080 \
  bash "$ROOT_DIR/scripts/install-server.sh" >"$INVALID_NAME_LOG" 2>&1; then
  echo "installer accepted unsafe NGINX_SERVER_NAME" >&2
  exit 1
fi
assert_contains "$INVALID_NAME_LOG" "invalid NGINX_SERVER_NAME"

[[ -x "$INSTALL_ROOT/bin/lobster-waku-gateway" ]] || {
  echo "missing installed gateway binary" >&2
  exit 1
}
[[ -f "$INSTALL_ROOT/web/index.html" ]] || {
  echo "missing installed web shell" >&2
  exit 1
}
[[ -f "$INSTALL_ROOT/release-manifest.json" && -f "$INSTALL_ROOT/web/release-manifest.json" ]] || {
  echo "missing installed release manifest" >&2
  exit 1
}
[[ -f "$SYSTEMD_UNIT" ]] || {
  echo "missing generated systemd unit" >&2
  exit 1
}
[[ -f "$NGINX_SITE_DEBIAN" ]] || {
  echo "missing generated nginx site" >&2
  exit 1
}
[[ -L "$NGINX_LINK_DEBIAN" ]] || {
  echo "missing nginx enabled symlink" >&2
  exit 1
}

assert_contains "$SYSTEMD_UNIT" "ExecStart=$INSTALL_ROOT/bin/lobster-waku-gateway --host 127.0.0.1 --port 8787 --state-dir $STATE_DIR"
assert_contains "$NGINX_SITE_DEBIAN" "root $INSTALL_ROOT/web;"
assert_contains "$NGINX_SITE_DEBIAN" "server_name \"$NGINX_SERVER_NAME\";"
assert_contains "$NGINX_SITE_DEBIAN" "proxy_pass http://127.0.0.1:8787;"
assert_contains "$NGINX_SITE_DEBIAN" "proxy_method GET;"
assert_contains "$LOG_FILE" "systemctl daemon-reload"
assert_contains "$LOG_FILE" "systemctl stop $SERVICE_NAME"
assert_contains "$LOG_FILE" "systemctl enable --now $SERVICE_NAME"
assert_contains "$LOG_FILE" "nginx -t"
assert_contains "$LOG_FILE" "systemctl enable --now nginx"
assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/health"
assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/v1/version"
assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/v1/provider"
assert_contains "$LOG_FILE" "curl http://127.0.0.1:8080/release-manifest.json"

echo "install layout smoke passed"
echo "state root: $STATE_ROOT"
