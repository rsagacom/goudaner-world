#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-resident-mainline.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing resident mainline smoke script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert "set -euo pipefail" in text
    assert 'PORT="${PORT:-}"' in text
    assert 'KEEP_STATE="${KEEP_STATE:-0}"' in text
    assert 'SKIP_BUILD="${SKIP_BUILD:-0}"' in text
    assert 'GATEWAY_BIN="${GATEWAY_BIN:-$ROOT_DIR/target/debug/lobster-waku-gateway}"' in text
    assert 'CLI_BIN="${CLI_BIN:-$ROOT_DIR/target/debug/lobster-cli}"' in text
    assert 'TUI_BIN="${TUI_BIN:-$ROOT_DIR/target/debug/lobster-tui}"' in text
    assert 'mktemp -d "${TMPDIR:-/tmp}/lobster-resident-smoke.XXXXXX"' in text
    assert "wait_for_health()" in text
    assert "need_cmd curl" in text
    assert "need_cmd python3" in text
    assert 'export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"' in text
    assert 'export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"' in text
    assert "reserve_port()" in text
    assert 'if [[ -z "$PORT" ]]; then' in text
    assert text.index('if [[ "$SKIP_BUILD" != "1" ]]; then') < text.index("need_cmd cargo")
    assert text.index("need_cmd cargo") < text.index('cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway -p lobster-cli -p lobster-tui')
    assert 'if [[ ! -x "$GATEWAY_BIN" ]]' in text
    assert 'echo "gateway binary not found: $GATEWAY_BIN" >&2' in text
    assert 'if [[ ! -x "$CLI_BIN" ]]' in text
    assert 'echo "cli binary not found: $CLI_BIN" >&2' in text
    assert 'if [[ ! -x "$TUI_BIN" ]]' in text
    assert 'echo "tui binary not found: $TUI_BIN" >&2' in text
    assert text.index('if [[ ! -x "$GATEWAY_BIN" ]]') > text.index('fi\n\nif [[ ! -x "$GATEWAY_BIN" ]]')
    assert text.index('STATE_ROOT="$(mktemp_dir)"') > text.index('if [[ ! -x "$TUI_BIN" ]]')
    assert 'export LOBSTER_WEB_GENERATED_DIR="$STATE_ROOT/web-generated"' in text
    assert 'cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway -p lobster-cli -p lobster-tui' in text
    assert "cleanup()" in text
    assert 'kill "$GATEWAY_PID"' in text
    assert 'rm -rf "$STATE_ROOT"' in text
    assert 'LOBSTER_DEV_EMAIL_OTP_INLINE=1 LOBSTER_DEV_AUTH_BYPASS=1 "$GATEWAY_BIN"' in text
    assert 'LOBSTER_SESSION_TOKEN="$session_token" ' + "\\" in text
    assert '"$GATEWAY_URL/health"' in text
    assert '"$GATEWAY_URL/v1/auth/preflight"' in text
    assert '"email":"novel.reader@example.com"' in text
    assert '"device_physical_address":"66:55:44:33:22:11"' in text
    assert '"$GATEWAY_URL/v1/auth/email-otp/request"' in text
    assert '"resident_id":"novel-reader"' in text
    assert "challenge_id" in text
    assert "dev_code" in text
    assert '"$GATEWAY_URL/v1/auth/email-otp/verify"' in text
    assert 'payload[\'resident_id\'] == \'novel-reader\'' in text
    assert "session_token" in text
    assert '"authorization: Bearer $session_token"' in text
    assert '"$GATEWAY_URL/v1/cities/join"' in text
    assert '"resident_id":"guest-01"' in text
    assert '[[ "$join_unregistered_status" == "400" ]]' in text
    assert "not registered" in text
    assert '"city":"core-harbor","resident_id":"novel-reader"' in text
    assert '"$GATEWAY_URL/v1/residents"' in text
    assert "active_cities" in text
    assert "$CLI_BIN rooms" in text
    assert '--token "$session_token"' in text
    assert 'room:city:core-harbor:lobby' in text
    assert 'dm:guide:novel-reader' in text
    assert 'LOBSTER_TUI_SMOKE_DUMP=json' in text
    assert '"$TUI_BIN" --mode direct' in text
    assert "ResidenceDirect" in text
    assert 'LOBSTER_TUI_SMOKE_DUMP=plain' in text
    assert "LOBSTER_TUI_SMOKE_SCRIPT" in text
    assert '"/dm $DM_PEER_ID"' in text
    assert '"/search $DM_TEXT"' in text
    assert 'tui_script_output="$(' in text
    assert 'grep -F "搜索「${DM_TEXT}」命中' in text
    assert '"$TUI_BIN" --mode user' in text
    assert "$CLI_BIN tail" in text
    assert "USER_RESIDENT_MAINLINE_SMOKE_首条消息" in text
    assert "USER_RESIDENT_DM_SMOKE_私帖首条消息" in text
    assert 'echo "resident mainline smoke passed"' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
