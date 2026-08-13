#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "install-server.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing install server script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert "set -euo pipefail" in text
    assert 'STATE_DIR="${STATE_DIR:-/var/lib/lobster-chat}"' in text
    assert 'INSTALL_ROOT="${INSTALL_ROOT:-/opt/lobster-chat}"' in text
    assert 'BIN_DIR="${BIN_DIR:-$INSTALL_ROOT/bin}"' in text
    assert 'WEB_DIR="${WEB_DIR:-$INSTALL_ROOT/web}"' in text
    assert 'BUILD_DIR="${BUILD_DIR:-$INSTALL_ROOT/build}"' in text
    assert 'CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$BUILD_DIR/target}"' in text
    assert 'SERVICE_NAME="${SERVICE_NAME:-lobster-waku-gateway}"' in text
    assert 'GATEWAY_ARTIFACT="${GATEWAY_ARTIFACT:-}"' in text
    assert 'WEB_ARTIFACT="${WEB_ARTIFACT:-}"' in text
    assert 'RELEASE_MANIFEST="${RELEASE_MANIFEST:-}"' in text
    assert 'LISTEN_HOST="${LISTEN_HOST:-127.0.0.1}"' in text
    assert 'LISTEN_PORT="${LISTEN_PORT:-8787}"' in text
    assert 'PUBLIC_PORT="${PUBLIC_PORT:-80}"' in text
    assert 'NGINX_SERVER_NAME="${NGINX_SERVER_NAME:-}"' in text
    assert 'HOST_TARGET_OVERRIDE="${HOST_TARGET_OVERRIDE:-}"' in text
    assert 'DEFAULT_SYSTEMD_UNIT="/etc/systemd/system/${SERVICE_NAME}.service"' in text
    assert 'DEFAULT_NGINX_SITE_DEBIAN="/etc/nginx/sites-available/lobster-chat"' in text
    assert 'DEFAULT_NGINX_SITE_RHEL="/etc/nginx/conf.d/lobster-chat.conf"' in text
    assert "need_cmd install" in text
    assert "need_cmd systemctl" in text
    assert "need_cmd nginx" in text
    assert "need_cmd tar" in text
    assert "need_cmd curl" in text
    assert "need_cmd python3" in text
    assert "validate_nginx_server_name()" in text
    assert "invalid NGINX_SERVER_NAME" in text
    assert "detect_target_triple()" in text
    assert 'Linux:x86_64) echo "x86_64-unknown-linux-gnu"' in text
    assert 'Darwin:arm64|Darwin:aarch64) echo "aarch64-apple-darwin"' in text
    assert "validate_gateway_artifact()" in text
    assert '"lobster-waku-gateway-${expected_target}.tar.gz"' in text
    assert "gateway artifact target mismatch" in text
    assert "install_gateway_from_artifact()" in text
    assert 'tar -xzf "$artifact_path" -C "$tmp_dir"' in text
    assert 'install -m 0755 "$tmp_dir/lobster-waku-gateway" "$BIN_DIR/lobster-waku-gateway"' in text
    assert "install_web_from_artifact()" in text
    assert 'cp -R "$tmp_dir/." "$WEB_DIR/"' in text
    assert "validate_release_manifest()" in text
    assert "gateway artifact checksum does not match release manifest" in text
    assert "web artifact checksum does not match release manifest" in text
    assert "RELEASE_MANIFEST is required when installing prebuilt artifacts" in text
    assert 'install -m 0644 "$RELEASE_MANIFEST" "$WEB_DIR/release-manifest.json"' in text
    assert "configure_rust_mirrors()" in text
    assert 'registry = "sparse+https://rsproxy.cn/index/"' in text
    assert "ensure_modern_rust()" in text
    assert 'local minimum="1.85.0"' in text
    assert "rustup toolchain install stable" in text
    assert "stop_conflicting_gateway_processes()" in text
    assert 'pattern="lobster-waku-gateway --host ${LISTEN_HOST} --port ${LISTEN_PORT}"' in text
    assert 'kill -9 "$pid"' in text
    assert "assert_listen_port_free()" in text
    assert 'ss -ltnp' in text
    assert 'host_target="$(detect_target_triple)"' in text
    assert 'validate_gateway_artifact "$GATEWAY_ARTIFACT" "$host_target"' in text
    assert "ensure_modern_rust" in text
    assert text.index('if [[ -n "$GATEWAY_ARTIFACT" ]]; then') < text.index('validate_gateway_artifact "$GATEWAY_ARTIFACT" "$host_target"')
    build_source_index = text.index('echo "== building gateway from source =="')
    ensure_call_index = text.index("ensure_modern_rust", build_source_index)
    assert text.index('if [[ -n "$WEB_ARTIFACT" && ! -f "$WEB_ARTIFACT" ]]; then') < ensure_call_index
    assert 'echo "== installing files =="' in text
    assert 'install -d "$BIN_DIR" "$WEB_DIR" "$STATE_DIR"' in text
    assert 'echo "== building gateway from source =="\n  ensure_modern_rust\n  cd "$ROOT_DIR"' in text
    assert 'cargo build --release -p lobster-waku-gateway' in text
    assert 'install -m 0755 "$CARGO_TARGET_DIR/release/lobster-waku-gateway" "$BIN_DIR/lobster-waku-gateway"' in text
    assert 'cp -R "$ROOT_DIR/apps/lobster-web-shell/." "$WEB_DIR/"' in text
    assert 'cat > "$SYSTEMD_UNIT" <<EOF' in text
    assert 'ExecStart=$BIN_DIR/lobster-waku-gateway --host $LISTEN_HOST --port $LISTEN_PORT --state-dir $STATE_DIR' in text
    assert "resolve_nginx_site_path()" in text
    assert 'cat > "$NGINX_SITE_PATH" <<EOF' in text
    assert 'listen $PUBLIC_PORT default_server;' in text
    assert 'server_name "$NGINX_SERVER_NAME";' in text
    assert "server_name _;" not in text
    assert 'proxy_pass http://$LISTEN_HOST:$LISTEN_PORT;' in text
    assert 'proxy_set_header Authorization \\$http_authorization;' in text
    assert 'location = /health' in text
    assert 'proxy_method GET;' in text
    assert 'try_files \\$uri \\$uri/ /index.html;' in text
    assert 'ln -sfn "$NGINX_SITE_PATH" "$NGINX_LINK_DEBIAN"' in text
    assert 'rm -f "$NGINX_DEFAULT_SITE_DEBIAN"' in text
    assert "systemctl daemon-reload" in text
    assert 'systemctl stop "$SERVICE_NAME"' in text
    assert "stop_conflicting_gateway_processes" in text
    assert "assert_listen_port_free" in text
    assert 'systemctl enable --now "$SERVICE_NAME"' in text
    assert "nginx -t" in text
    assert "systemctl reload nginx" in text
    assert "systemctl enable --now nginx" in text
    assert 'curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/health"' in text
    assert 'version_json="$(curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/v1/version")"' in text
    assert 'runtime_git_sha="$(printf \'%s\' "$version_json"' in text
    assert "running gateway git_sha does not match release manifest" in text
    assert 'curl -fsS "http://127.0.0.1:$PUBLIC_PORT/release-manifest.json"' in text
    assert 'curl -fsS "http://$LISTEN_HOST:$LISTEN_PORT/v1/provider"' in text
    assert 'echo "install complete"' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
