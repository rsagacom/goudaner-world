#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-install-layout.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing install layout smoke script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert 'KEEP_STATE="${KEEP_STATE:-0}"' in text
    assert 'mktemp -d "${TMPDIR:-/tmp}/lobster-install-smoke.XXXXXX"' in text
    assert 'FAKE_BIN="$STATE_ROOT/fake-bin"' in text
    assert 'ARTIFACT_ROOT="$STATE_ROOT/artifacts"' in text
    assert 'INSTALL_ROOT="$STATE_ROOT/install-root"' in text
    assert 'STATE_DIR="$STATE_ROOT/state"' in text
    assert 'SERVICE_NAME="lobster-waku-gateway-smoke"' in text
    assert 'GATEWAY_ARTIFACT="$ARTIFACT_ROOT/lobster-waku-gateway-${HOST_TARGET}.tar.gz"' in text
    assert 'WEB_ARTIFACT="$ARTIFACT_ROOT/lobster-web-shell.tar.gz"' in text
    assert 'RELEASE_MANIFEST="$ARTIFACT_ROOT/release-manifest.json"' in text
    assert 'SMOKE_GIT_SHA="1111111111111111111111111111111111111111"' in text
    assert 'cat >"$FAKE_BIN/systemctl"' in text
    assert 'cat >"$FAKE_BIN/nginx"' in text
    assert 'cat >"$FAKE_BIN/curl"' in text
    assert 'tar -czf "$GATEWAY_ARTIFACT"' in text
    assert 'tar -czf "$WEB_ARTIFACT"' in text
    assert 'sha256_file()' in text
    assert '>"$RELEASE_MANIFEST"' in text
    assert 'RELEASE_MANIFEST="$RELEASE_MANIFEST"' in text
    assert 'bash "$ROOT_DIR/scripts/install-server.sh"' in text
    assert '[[ -x "$INSTALL_ROOT/bin/lobster-waku-gateway" ]]' in text
    assert '[[ -f "$INSTALL_ROOT/web/index.html" ]]' in text
    assert '[[ -f "$INSTALL_ROOT/release-manifest.json" && -f "$INSTALL_ROOT/web/release-manifest.json" ]]' in text
    assert '[[ -f "$SYSTEMD_UNIT" ]]' in text
    assert '[[ -f "$NGINX_SITE_DEBIAN" ]]' in text
    assert '[[ -L "$NGINX_LINK_DEBIAN" ]]' in text
    assert 'assert_contains "$SYSTEMD_UNIT" "ExecStart=$INSTALL_ROOT/bin/lobster-waku-gateway --host 127.0.0.1 --port 8787 --state-dir $STATE_DIR"' in text
    assert 'assert_contains "$NGINX_SITE_DEBIAN" "root $INSTALL_ROOT/web;"' in text
    assert 'assert_contains "$NGINX_SITE_DEBIAN" "proxy_pass http://127.0.0.1:8787;"' in text
    assert 'assert_contains "$LOG_FILE" "systemctl enable --now $SERVICE_NAME"' in text
    assert 'assert_contains "$LOG_FILE" "nginx -t"' in text
    assert 'assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/health"' in text
    assert 'assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/v1/version"' in text
    assert 'assert_contains "$LOG_FILE" "curl http://127.0.0.1:8787/v1/provider"' in text
    assert 'assert_contains "$LOG_FILE" "curl http://127.0.0.1:8080/release-manifest.json"' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
