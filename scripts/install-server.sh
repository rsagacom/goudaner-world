#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="${STATE_DIR:-/var/lib/lobster-chat}"
INSTALL_ROOT="${INSTALL_ROOT:-/opt/lobster-chat}"
BIN_DIR="${BIN_DIR:-$INSTALL_ROOT/bin}"
WEB_DIR="${WEB_DIR:-$INSTALL_ROOT/web}"
BUILD_DIR="${BUILD_DIR:-$INSTALL_ROOT/build}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$BUILD_DIR/target}"
SERVICE_NAME="${SERVICE_NAME:-lobster-waku-gateway}"
GATEWAY_ARTIFACT="${GATEWAY_ARTIFACT:-}"
WEB_ARTIFACT="${WEB_ARTIFACT:-}"
RELEASE_MANIFEST="${RELEASE_MANIFEST:-}"
LISTEN_HOST="${LISTEN_HOST:-127.0.0.1}"
LISTEN_PORT="${LISTEN_PORT:-8787}"
PUBLIC_PORT="${PUBLIC_PORT:-80}"
NGINX_SERVER_NAME="${NGINX_SERVER_NAME:-}"
HOST_TARGET_OVERRIDE="${HOST_TARGET_OVERRIDE:-}"
DEFAULT_SYSTEMD_UNIT="/etc/systemd/system/${SERVICE_NAME}.service"
DEFAULT_NGINX_SITE_DEBIAN="/etc/nginx/sites-available/lobster-chat"
DEFAULT_NGINX_LINK_DEBIAN="/etc/nginx/sites-enabled/lobster-chat"
DEFAULT_NGINX_DEFAULT_SITE_DEBIAN="/etc/nginx/sites-enabled/default"
DEFAULT_NGINX_SITE_RHEL="/etc/nginx/conf.d/lobster-chat.conf"
SYSTEMD_UNIT="${SYSTEMD_UNIT:-$DEFAULT_SYSTEMD_UNIT}"
NGINX_SITE_DEBIAN="${NGINX_SITE_DEBIAN:-$DEFAULT_NGINX_SITE_DEBIAN}"
NGINX_LINK_DEBIAN="${NGINX_LINK_DEBIAN:-$DEFAULT_NGINX_LINK_DEBIAN}"
NGINX_DEFAULT_SITE_DEBIAN="${NGINX_DEFAULT_SITE_DEBIAN:-$DEFAULT_NGINX_DEFAULT_SITE_DEBIAN}"
NGINX_SITE_RHEL="${NGINX_SITE_RHEL:-$DEFAULT_NGINX_SITE_RHEL}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

validate_nginx_server_name() {
  if [[ -z "$NGINX_SERVER_NAME" ]]; then
    return
  fi

  if (( ${#NGINX_SERVER_NAME} > 253 )) \
    || [[ "$NGINX_SERVER_NAME" == *[!A-Za-z0-9._-]* ]] \
    || [[ "$NGINX_SERVER_NAME" == .* ]] \
    || [[ "$NGINX_SERVER_NAME" == -* ]] \
    || [[ "$NGINX_SERVER_NAME" == *..* ]]; then
    echo "invalid NGINX_SERVER_NAME: expected one DNS name without whitespace or nginx syntax" >&2
    exit 1
  fi
}

cargo_version_number() {
  cargo --version 2>/dev/null | awk '{ print $2 }'
}

version_ge() {
  local current="$1"
  local minimum="$2"
  [[ -n "$current" ]] || return 1
  printf '%s\n%s\n' "$minimum" "$current" | sort -V -C
}

detect_target_triple() {
  if [[ -n "$HOST_TARGET_OVERRIDE" ]]; then
    echo "$HOST_TARGET_OVERRIDE"
    return
  fi
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os:$arch" in
    Linux:x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Linux:aarch64|Linux:arm64) echo "aarch64-unknown-linux-gnu" ;;
    Darwin:x86_64) echo "x86_64-apple-darwin" ;;
    Darwin:arm64|Darwin:aarch64) echo "aarch64-apple-darwin" ;;
    *)
      echo "unsupported host target: ${os}/${arch}" >&2
      exit 1
      ;;
  esac
}

validate_gateway_artifact() {
  local artifact_path="$1"
  local expected_target="$2"
  local base_name
  base_name="$(basename "$artifact_path")"
  if [[ ! -f "$artifact_path" ]]; then
    echo "gateway artifact not found: $artifact_path" >&2
    exit 1
  fi
  if [[ "$base_name" != "lobster-waku-gateway-${expected_target}.tar.gz" ]]; then
    cat >&2 <<EOF
gateway artifact target mismatch:
  expected: lobster-waku-gateway-${expected_target}.tar.gz
  got:      ${base_name}
EOF
    exit 1
  fi
}

install_gateway_from_artifact() {
  local artifact_path="$1"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  tar -xzf "$artifact_path" -C "$tmp_dir"
  if [[ ! -x "$tmp_dir/lobster-waku-gateway" ]]; then
    echo "artifact did not contain executable lobster-waku-gateway" >&2
    rm -rf "$tmp_dir"
    exit 1
  fi
  install -m 0755 "$tmp_dir/lobster-waku-gateway" "$BIN_DIR/lobster-waku-gateway"
  rm -rf "$tmp_dir"
}

install_web_from_artifact() {
  local artifact_path="$1"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  tar -xzf "$artifact_path" -C "$tmp_dir"
  rm -rf "$WEB_DIR"
  install -d "$WEB_DIR"
  cp -R "$tmp_dir/." "$WEB_DIR/"
  rm -rf "$tmp_dir"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  else
    need_cmd shasum
    shasum -a 256 "$1" | awk '{ print $1 }'
  fi
}

manifest_value() {
  python3 - "$RELEASE_MANIFEST" "$1" <<'PY'
import json
import sys

payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
value = payload
for part in sys.argv[2].split("."):
    value = value[part]
print(value)
PY
}

validate_release_manifest() {
  [[ -f "$RELEASE_MANIFEST" ]] || {
    echo "release manifest not found: $RELEASE_MANIFEST" >&2
    exit 1
  }
  local manifest_git_sha
  manifest_git_sha="$(manifest_value git_sha)"
  [[ "$manifest_git_sha" =~ ^[0-9a-fA-F]{40}$ ]] || {
    echo "release manifest has invalid git_sha" >&2
    exit 1
  }
  if [[ -n "$GATEWAY_ARTIFACT" ]]; then
    [[ "$(sha256_file "$GATEWAY_ARTIFACT")" == "$(manifest_value artifacts.gateway.sha256)" ]] || {
      echo "gateway artifact checksum does not match release manifest" >&2
      exit 1
    }
  fi
  if [[ -n "$WEB_ARTIFACT" ]]; then
    [[ "$(sha256_file "$WEB_ARTIFACT")" == "$(manifest_value artifacts.web.sha256)" ]] || {
      echo "web artifact checksum does not match release manifest" >&2
      exit 1
    }
  fi
}

configure_rust_mirrors() {
  local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  mkdir -p "$cargo_home"
  cat > "${cargo_home}/config.toml" <<'EOF'
[source.crates-io]
replace-with = "rsproxy-sparse"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
EOF

  export RUSTUP_DIST_SERVER="${RUSTUP_DIST_SERVER:-https://rsproxy.cn}"
  export RUSTUP_UPDATE_ROOT="${RUSTUP_UPDATE_ROOT:-https://rsproxy.cn/rustup}"
}

ensure_modern_rust() {
  local minimum="1.85.0"
  local cargo_home_bin="$HOME/.cargo/bin"

  if [ -x "$cargo_home_bin/cargo" ]; then
    export PATH="$cargo_home_bin:$PATH"
  elif command -v rustup >/dev/null 2>&1; then
    export PATH="$cargo_home_bin:$PATH"
  fi

  if command -v cargo >/dev/null 2>&1 && version_ge "$(cargo_version_number)" "$minimum"; then
    echo "== rust toolchain ok: $(cargo --version) =="
    return
  fi

  echo "== bootstrapping modern rust toolchain (need cargo >= ${minimum}) =="
  need_cmd curl
  need_cmd sh
  configure_rust_mirrors
  export RUSTUP_INIT_SKIP_PATH_CHECK=yes

  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
    sh /tmp/rustup-init.sh -y
  fi

  export PATH="$cargo_home_bin:$PATH"
  rustup toolchain install stable
  rustup default stable

  if ! command -v cargo >/dev/null 2>&1 || ! version_ge "$(cargo_version_number)" "$minimum"; then
    echo "cargo is still too old after rustup bootstrap" >&2
    exit 1
  fi

  echo "== rust toolchain upgraded: $(cargo --version) =="
}

need_cmd install
need_cmd mkdir
need_cmd awk
need_cmd sort
need_cmd systemctl
need_cmd nginx
need_cmd tar
need_cmd uname
need_cmd curl
need_cmd python3
validate_nginx_server_name

stop_conflicting_gateway_processes() {
  if ! command -v pgrep >/dev/null 2>&1; then
    return
  fi

  local pattern pid cmd
  pattern="lobster-waku-gateway --host ${LISTEN_HOST} --port ${LISTEN_PORT}"
  while IFS= read -r line; do
    pid="${line%% *}"
    cmd="${line#* }"
    [[ -n "$pid" && -n "$cmd" ]] || continue
    case "$cmd" in
      "$BIN_DIR/lobster-waku-gateway"*) continue ;;
    esac
    echo "== stopping conflicting gateway process: $cmd =="
    kill "$pid" >/dev/null 2>&1 || true
    sleep 1
    kill -0 "$pid" >/dev/null 2>&1 && kill -9 "$pid" >/dev/null 2>&1 || true
  done < <(pgrep -af "$pattern" || true)
}

assert_listen_port_free() {
  if ! command -v ss >/dev/null 2>&1; then
    return
  fi

  local listeners
  listeners="$(ss -ltnp 2>/dev/null | grep "${LISTEN_HOST}:${LISTEN_PORT}" || true)"
  if [[ -n "$listeners" ]]; then
    echo "listen address ${LISTEN_HOST}:${LISTEN_PORT} is still busy after cleanup" >&2
    echo "$listeners" >&2
    exit 1
  fi
}

host_target="$(detect_target_triple)"
echo "== target triple: ${host_target} =="

if [[ -n "$GATEWAY_ARTIFACT" ]]; then
  validate_gateway_artifact "$GATEWAY_ARTIFACT" "$host_target"
fi

if [[ -n "$WEB_ARTIFACT" && ! -f "$WEB_ARTIFACT" ]]; then
  echo "web artifact not found: $WEB_ARTIFACT" >&2
  exit 1
fi

if [[ -n "$GATEWAY_ARTIFACT$WEB_ARTIFACT" && -z "$RELEASE_MANIFEST" ]]; then
  echo "RELEASE_MANIFEST is required when installing prebuilt artifacts" >&2
  exit 1
fi

if [[ -n "$RELEASE_MANIFEST" ]]; then
  validate_release_manifest
fi

resolve_nginx_site_path() {
  local debian_site_dir debian_link_dir rhel_dir
  debian_site_dir="$(dirname "$NGINX_SITE_DEBIAN")"
  debian_link_dir="$(dirname "$NGINX_LINK_DEBIAN")"
  rhel_dir="$(dirname "$NGINX_SITE_RHEL")"

  if [[ -d "$debian_site_dir" ]] || [[ -d "$debian_link_dir" ]] || [[ "$NGINX_SITE_DEBIAN" != "$DEFAULT_NGINX_SITE_DEBIAN" ]] || [[ "$NGINX_LINK_DEBIAN" != "$DEFAULT_NGINX_LINK_DEBIAN" ]]; then
    mkdir -p "$debian_site_dir" "$debian_link_dir"
    echo "$NGINX_SITE_DEBIAN"
  else
    mkdir -p "$rhel_dir"
    echo "$NGINX_SITE_RHEL"
  fi
}

echo "== installing files =="
install -d "$BIN_DIR" "$WEB_DIR" "$STATE_DIR"
if [[ -n "$GATEWAY_ARTIFACT" ]]; then
  echo "== installing gateway from artifact =="
  install_gateway_from_artifact "$GATEWAY_ARTIFACT"
else
  echo "== building gateway from source =="
  ensure_modern_rust
  cd "$ROOT_DIR"
  install -d "$BUILD_DIR"
  export CARGO_TARGET_DIR
  cargo build --release -p lobster-waku-gateway
  install -m 0755 "$CARGO_TARGET_DIR/release/lobster-waku-gateway" "$BIN_DIR/lobster-waku-gateway"
fi
if [[ -n "$WEB_ARTIFACT" ]]; then
  echo "== installing web shell from artifact =="
  install_web_from_artifact "$WEB_ARTIFACT"
else
  echo "== installing web shell from workspace =="
  cp -R "$ROOT_DIR/apps/lobster-web-shell/." "$WEB_DIR/"
fi
if [[ -n "$RELEASE_MANIFEST" ]]; then
  install -m 0644 "$RELEASE_MANIFEST" "$INSTALL_ROOT/release-manifest.json"
  install -m 0644 "$RELEASE_MANIFEST" "$WEB_DIR/release-manifest.json"
fi

echo "== writing systemd unit =="
mkdir -p "$(dirname "$SYSTEMD_UNIT")"
cat > "$SYSTEMD_UNIT" <<EOF
[Unit]
Description=Lobster Chat Waku Gateway
After=network.target

[Service]
Type=simple
ExecStart=$BIN_DIR/lobster-waku-gateway --host $LISTEN_HOST --port $LISTEN_PORT --state-dir $STATE_DIR
Restart=always
RestartSec=2
WorkingDirectory=$INSTALL_ROOT

[Install]
WantedBy=multi-user.target
EOF

NGINX_SITE_PATH="$(resolve_nginx_site_path)"

echo "== writing nginx site =="
mkdir -p "$(dirname "$NGINX_SITE_PATH")"
cat > "$NGINX_SITE_PATH" <<EOF
server {
    listen $PUBLIC_PORT default_server;
    listen [::]:$PUBLIC_PORT default_server;
    server_name "$NGINX_SERVER_NAME";

    root $WEB_DIR;
    index index.html;

    location /v1/ {
        proxy_pass http://$LISTEN_HOST:$LISTEN_PORT;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header Authorization \$http_authorization;
    }

    location = /health {
        proxy_method GET;
        proxy_pass_request_body off;
        proxy_set_header Content-Length "";
        proxy_pass http://$LISTEN_HOST:$LISTEN_PORT/health;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
    }

    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # JS/CSS/HTML 禁用长缓存:源站发 no-cache,CDN(CF 默认遵循源站)
    # 每次回源校验,避免热更新后边缘节点继续发旧模块导致功能不一致。
    location ~* \.(?:js|mjs|css|html)\$ {
        add_header Cache-Control "no-cache";
        try_files \$uri \$uri/ /index.html;
    }
}
EOF

if [[ "$NGINX_SITE_PATH" == "$NGINX_SITE_DEBIAN" ]]; then
  ln -sfn "$NGINX_SITE_PATH" "$NGINX_LINK_DEBIAN"
  rm -f "$NGINX_DEFAULT_SITE_DEBIAN"
fi

echo "== enabling services =="
systemctl daemon-reload
systemctl stop "$SERVICE_NAME" >/dev/null 2>&1 || true
stop_conflicting_gateway_processes
assert_listen_port_free
systemctl enable --now "$SERVICE_NAME"
nginx -t
if systemctl is-active --quiet nginx; then
  systemctl reload nginx
else
  systemctl enable --now nginx
fi

echo "== health checks =="
curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/health" && echo
version_json="$(curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/v1/version")"
printf '%s\n' "$version_json"
curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/v1/provider" && echo
if [[ -n "$RELEASE_MANIFEST" ]]; then
  runtime_git_sha="$(printf '%s' "$version_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["git_sha"])')"
  manifest_git_sha="$(manifest_value git_sha)"
  if [[ "$runtime_git_sha" != "$manifest_git_sha" ]]; then
    echo "running gateway git_sha does not match release manifest" >&2
    exit 1
  fi
  curl -fsS "http://127.0.0.1:$PUBLIC_PORT/release-manifest.json" && echo
fi

echo "install complete"
echo "gateway: http://$LISTEN_HOST:$LISTEN_PORT"
echo "public:   http://<server-ip>:$PUBLIC_PORT/"
