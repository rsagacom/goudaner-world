#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-provider-federation.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing provider federation smoke script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert 'GATEWAY_ARTIFACT="${GATEWAY_ARTIFACT:-}"' in text
    assert 'SKIP_BUILD="${SKIP_BUILD:-0}"' in text
    assert 'UPSTREAM_PORT="${UPSTREAM_PORT:-}"' in text
    assert 'DOWNSTREAM_PORT="${DOWNSTREAM_PORT:-}"' in text
    assert 'reserve_port() {' in text
    assert 'need_cmd curl\nneed_cmd grep\nneed_cmd mktemp\nneed_cmd tar\nneed_cmd python3' in text
    assert 'export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"' in text
    assert 'export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"' in text
    assert 'if [[ -z "$UPSTREAM_PORT" ]]; then' in text
    assert 'if [[ -z "$DOWNSTREAM_PORT" ]]; then' in text
    assert text.index('if [[ "$SKIP_BUILD" != "1" && -z "$GATEWAY_ARTIFACT" ]]; then') < text.index("need_cmd cargo")
    assert text.index("need_cmd cargo") < text.index('cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway')
    assert 'tar -xzf "$GATEWAY_ARTIFACT" -C "$EXTRACT_DIR"' in text
    assert 'BIN_PATH="$EXTRACT_DIR/lobster-waku-gateway"' in text
    assert text.index('if [[ ! -x "$BIN_PATH" ]]') < text.index('STATE_ROOT="$(mktemp_dir)"')
    assert 'cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway' in text
    assert '"$BIN_PATH" \\' in text
    assert text.count('LOBSTER_DEV_AUTH_BYPASS=1 "$BIN_PATH"') == 2, (
        "provider federation fixture must explicitly enable dev auth bypass for both synthetic gateways"
    )
    assert '--upstream-gateway-url "http://$HOST:$UPSTREAM_PORT"' in text
    assert 'wait_for_health "upstream" "http://$HOST:$UPSTREAM_PORT/health"' in text
    assert 'wait_for_health "downstream" "http://$HOST:$DOWNSTREAM_PORT/health"' in text
    assert text.index('wait_for_health "upstream"') < text.index('echo "== starting downstream gateway')
    assert 'tail -n 80 "$DOWNSTREAM_LOG"' in text
    assert 'curl -fsS "http://$HOST:$DOWNSTREAM_PORT/v1/provider"' in text
    assert 'grep -q \'"mode":"remote-gateway"\'' in text
    assert 'grep -q \'"reachable":true\'' in text
    assert '"http://$HOST:$DOWNSTREAM_PORT/v1/shell/message"' in text
    assert 'curl -fsS "http://$HOST:$UPSTREAM_PORT/v1/shell/state"' in text
    assert 'grep -Fq "$message"' in text
    assert 'rm -rf "$EXTRACT_DIR"' in text
    assert 'kill "$DOWNSTREAM_PID"' in text
    assert 'kill "$UPSTREAM_PID"' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
