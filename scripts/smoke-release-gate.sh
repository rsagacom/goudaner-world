#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_PREFLIGHT="${RUN_PREFLIGHT:-1}"
INCLUDE_PROVIDER_FEDERATION="${INCLUDE_PROVIDER_FEDERATION:-${WITH_PROVIDER_FEDERATION:-1}}"
KEEP_STATE="${KEEP_STATE:-0}"
SKIP_BUILD="${SKIP_BUILD:-0}"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing command: $1" >&2
    exit 1
  }
}

run_step() {
  local label="$1"
  shift
  echo "== $label =="
  "$@"
  echo
}

run_shell_step() {
  local label="$1"
  local script="$2"
  echo "== $label =="
  bash "$script"
  echo
}

need_cmd bash
need_cmd python3
need_cmd node

# Local smoke gateways must bypass any user-configured HTTP proxy.
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"
export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"

run_step "scripts quick unit coverage" python3 "$ROOT_DIR/scripts/test_scripts_quick_unit_coverage.py"
run_step "backup state unit" python3 "$ROOT_DIR/scripts/test_backup_state_unit.py"
run_step "smoke runtime contract unit" python3 "$ROOT_DIR/scripts/test_smoke_runtime_contract_unit.py"
run_step "preflight unit" python3 "$ROOT_DIR/scripts/test_preflight_unit.py"

if [[ "$RUN_PREFLIGHT" == "1" ]]; then
  run_shell_step "preflight" "$ROOT_DIR/scripts/preflight.sh"
fi

if [[ "$SKIP_BUILD" != "1" ]]; then
  need_cmd cargo
  run_step \
    "rust fmt" \
    cargo fmt --manifest-path "$ROOT_DIR/Cargo.toml" --all --check
  run_step \
    "rust workspace tests" \
    cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --quiet
  run_step \
    "rust lint" \
    cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" --workspace -- -D warnings
  run_step \
    "building shared debug binaries" \
    cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway -p lobster-cli -p lobster-tui
fi

export KEEP_STATE
export SKIP_BUILD=1
export LOBSTER_CHAT_ROOT="${LOBSTER_CHAT_ROOT:-$ROOT_DIR}"
gateway_bin_default="$ROOT_DIR/target/debug/lobster-waku-gateway"
export GATEWAY_BIN="${GATEWAY_BIN:-$gateway_bin_default}"
export CLI_BIN="${CLI_BIN:-$(dirname "$GATEWAY_BIN")/lobster-cli}"
export TUI_BIN="${TUI_BIN:-$(dirname "$GATEWAY_BIN")/lobster-tui}"

run_step "makefile smoke unit" python3 "$ROOT_DIR/scripts/test_makefile_unit.py"
run_step "cli channel smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_cli_channel_unit.py"
run_shell_step "cli channel smoke" "$ROOT_DIR/scripts/smoke-cli-channel.sh"
run_step "auth registration smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_auth_registration_unit.py"
run_shell_step "auth registration smoke" "$ROOT_DIR/scripts/smoke-auth-registration.sh"
run_step "resident mainline smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_resident_mainline_unit.py"
run_shell_step "resident mainline smoke" "$ROOT_DIR/scripts/smoke-resident-mainline.sh"
run_step "shell dual HTTP smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_shell_dual_http_unit.py"
run_shell_step "shell dual HTTP smoke" "$ROOT_DIR/scripts/smoke-shell-dual-http.sh"
run_step "shell direct HTTP smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_shell_direct_http_unit.py"
run_shell_step "shell direct HTTP smoke" "$ROOT_DIR/scripts/smoke-shell-direct-http.sh"
run_step "web shell smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_web_shell_unit.py"
run_shell_step "web shell smoke" "$ROOT_DIR/scripts/smoke-web-shell.sh"
run_step "web dual browser smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_web_dual_browser_unit.py"
run_step "web dual browser smoke" node "$ROOT_DIR/scripts/smoke-web-dual-browser.mjs"
run_step "terminal smoke unit" python3 "$ROOT_DIR/scripts/test_start_terminal_unit.py"
run_step "start terminal shell unit" python3 "$ROOT_DIR/scripts/test_start_terminal_shell_unit.py"
run_step "terminal smoke" python3 "$ROOT_DIR/scripts/test_start_terminal.py"

if [[ "$INCLUDE_PROVIDER_FEDERATION" == "1" ]]; then
  run_step "provider federation smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_provider_federation_unit.py"
  run_shell_step "provider federation smoke" "$ROOT_DIR/scripts/smoke-provider-federation.sh"
fi

run_step "install server unit" python3 "$ROOT_DIR/scripts/test_install_server_unit.py"
run_step "install layout smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_install_layout_unit.py"
run_step "public ingress smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_public_ingress_unit.py"
run_step "package release unit" python3 "$ROOT_DIR/scripts/test_package_release_unit.py"
run_step "release workflow unit" python3 "$ROOT_DIR/scripts/test_release_workflow_unit.py"
run_step "production readiness unit" python3 "$ROOT_DIR/scripts/test_production_readiness_unit.py"
run_step "restart gateway unit" python3 "$ROOT_DIR/scripts/test_restart_gateway_unit.py"
run_step "rust production panic scan unit" python3 "$ROOT_DIR/scripts/test_rust_production_panic_scan_unit.py"
run_step "rust production panic scan" python3 "$ROOT_DIR/scripts/rust-production-panic-scan.py"
run_step "web preview unit" python3 "$ROOT_DIR/scripts/test_start_web_preview_unit.py"
run_step "preview server unit" python3 "$ROOT_DIR/scripts/test_preview_server_unit.py"
run_step "device id unit" python3 "$ROOT_DIR/scripts/test_lobster_device_id_unit.py"
run_step "web assets audit unit" python3 "$ROOT_DIR/scripts/test_audit_web_assets_unit.py"
run_step "complete verification unit" python3 "$ROOT_DIR/scripts/test_verify_complete_unit.py"

echo "== release gate passed =="
echo "root: $ROOT_DIR"
if [[ "$INCLUDE_PROVIDER_FEDERATION" == "1" ]]; then
  echo "provider interlink smoke: included"
else
  echo "provider interlink smoke: skipped"
fi
