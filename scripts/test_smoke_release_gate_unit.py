#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-release-gate.sh"
CI_WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"


def main() -> int:
    assert SCRIPT.exists(), f"missing release gate script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")
    assert CI_WORKFLOW.exists(), f"missing CI workflow: {CI_WORKFLOW}"
    ci_text = CI_WORKFLOW.read_text(encoding="utf-8")

    assert "smoke-auth-registration.sh" in text
    assert "smoke-resident-mainline.sh" in text
    assert "smoke-shell-dual-http.sh" in text
    assert "smoke-shell-direct-http.sh" in text
    assert "smoke-cli-channel.sh" in text
    assert "test_scripts_quick_unit_coverage.py" in text
    assert "test_preflight_unit.py" in text
    assert "test_makefile_unit.py" in text
    assert "test_smoke_cli_channel_unit.py" in text
    assert "test_smoke_auth_registration_unit.py" in text
    assert "test_smoke_resident_mainline_unit.py" in text
    assert "test_smoke_shell_dual_http_unit.py" in text
    assert "test_smoke_shell_direct_http_unit.py" in text
    assert "test_smoke_web_shell_unit.py" in text
    assert "test_smoke_web_dual_browser_unit.py" in text
    assert "test_start_terminal_unit.py" in text
    assert "test_start_terminal_shell_unit.py" in text
    assert "test_start_terminal.py" in text
    assert "test_smoke_provider_federation_unit.py" in text
    assert "test_install_server_unit.py" in text
    assert "test_smoke_install_layout_unit.py" in text
    assert "test_smoke_public_ingress_unit.py" in text
    assert "test_package_release_unit.py" in text
    assert "test_restart_gateway_unit.py" in text
    assert "test_rust_production_panic_scan_unit.py" in text
    assert "test_start_web_preview_unit.py" in text
    assert "test_preview_server_unit.py" in text
    assert "test_lobster_device_id_unit.py" in text
    assert "test_audit_web_assets_unit.py" in text
    assert "test_verify_complete_unit.py" in text
    assert "smoke-provider-federation.sh" in text
    assert "run_shell_step()" in text
    assert 'run_step "preflight unit" python3 "$ROOT_DIR/scripts/test_preflight_unit.py"' in text
    assert 'run_step "scripts quick unit coverage" python3 "$ROOT_DIR/scripts/test_scripts_quick_unit_coverage.py"' in text
    assert 'run_step "smoke runtime contract unit" python3 "$ROOT_DIR/scripts/test_smoke_runtime_contract_unit.py"' in text
    assert text.index('run_step "scripts quick unit coverage"') < text.index('run_step "preflight unit"')
    assert text.index('run_step "smoke runtime contract unit"') < text.index('run_step "preflight unit"')
    assert text.index('run_step "preflight unit"') < text.index('run_shell_step "preflight"')
    assert 'run_shell_step "preflight" "$ROOT_DIR/scripts/preflight.sh"' in text
    assert "need_cmd bash\nneed_cmd python3" in text
    assert text.index('if [[ "$SKIP_BUILD" != "1" ]]; then') < text.index("need_cmd cargo")
    assert 'run_step \\\n    "rust fmt" \\\n    cargo fmt --manifest-path "$ROOT_DIR/Cargo.toml" --all --check' in text
    assert "cargo fmt --all -- --check" in ci_text
    assert "cargo clippy --workspace -- -D warnings" in ci_text
    assert 'export NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"' in text
    assert 'export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"' in text
    assert text.index("need_cmd cargo") < text.index('"rust fmt"')
    assert '"rust workspace tests"' in text
    assert '"native Waku REST adapter tests"' in text
    assert 'cargo test --manifest-path "$ROOT_DIR/Cargo.toml" -p transport-waku --features native-waku-rest --quiet' in text
    assert 'run_step \\\n    "rust lint" \\\n    cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" --workspace -- -D warnings' in text
    assert '"native Waku REST adapter lint"' in text
    assert 'cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" -p transport-waku --features native-waku-rest -- -D warnings' in text
    assert text.index('"rust workspace tests"') < text.index('"native Waku REST adapter tests"')
    assert text.index('"native Waku REST adapter tests"') < text.index('"rust lint"')
    assert text.index('"rust lint"') < text.index('"native Waku REST adapter lint"')
    assert text.index('"rust fmt"') < text.index('"rust workspace tests"')
    assert text.index('"native Waku REST adapter lint"') < text.index('cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p lobster-waku-gateway -p lobster-cli -p lobster-tui')
    assert 'run_step "makefile smoke unit" python3 "$ROOT_DIR/scripts/test_makefile_unit.py"' in text
    assert 'run_shell_step "cli channel smoke" "$ROOT_DIR/scripts/smoke-cli-channel.sh"' in text
    assert 'run_step "cli channel smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_cli_channel_unit.py"' in text
    assert 'run_step "auth registration smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_auth_registration_unit.py"' in text
    assert text.index('run_step "auth registration smoke unit"') < text.index('run_shell_step "auth registration smoke"')
    assert 'run_shell_step "auth registration smoke" "$ROOT_DIR/scripts/smoke-auth-registration.sh"' in text
    assert 'run_step "resident mainline smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_resident_mainline_unit.py"' in text
    assert text.index('run_step "resident mainline smoke unit"') < text.index('run_shell_step "resident mainline smoke"')
    assert 'run_shell_step "resident mainline smoke" "$ROOT_DIR/scripts/smoke-resident-mainline.sh"' in text
    assert 'run_step "shell dual HTTP smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_shell_dual_http_unit.py"' in text
    assert text.index('run_step "shell dual HTTP smoke unit"') < text.index('run_shell_step "shell dual HTTP smoke"')
    assert 'run_shell_step "shell dual HTTP smoke" "$ROOT_DIR/scripts/smoke-shell-dual-http.sh"' in text
    assert 'run_step "shell direct HTTP smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_shell_direct_http_unit.py"' in text
    assert text.index('run_step "shell direct HTTP smoke unit"') < text.index('run_shell_step "shell direct HTTP smoke"')
    assert 'run_shell_step "shell direct HTTP smoke" "$ROOT_DIR/scripts/smoke-shell-direct-http.sh"' in text
    assert 'run_step "web shell smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_web_shell_unit.py"' in text
    assert text.index('run_step "web shell smoke unit"') < text.index('run_shell_step "web shell smoke"')
    assert 'run_shell_step "web shell smoke" "$ROOT_DIR/scripts/smoke-web-shell.sh"' in text
    assert 'run_step "web dual browser smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_web_dual_browser_unit.py"' in text
    assert text.index('run_step "web dual browser smoke unit"') < text.index('run_step "terminal smoke unit"')
    assert 'run_step "web dual browser smoke" node "$ROOT_DIR/scripts/smoke-web-dual-browser.mjs"' in text
    assert text.index('run_step "web dual browser smoke unit"') < text.index('run_step "web dual browser smoke"')
    assert text.index('run_step "web dual browser smoke"') < text.index('run_step "terminal smoke unit"')
    assert 'run_step "terminal smoke unit" python3 "$ROOT_DIR/scripts/test_start_terminal_unit.py"' in text
    assert text.index('run_step "terminal smoke unit"') < text.index('run_step "terminal smoke"')
    assert 'run_step "start terminal shell unit" python3 "$ROOT_DIR/scripts/test_start_terminal_shell_unit.py"' in text
    assert text.index('run_step "start terminal shell unit"') < text.index('run_step "terminal smoke"')
    assert 'run_step "provider federation smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_provider_federation_unit.py"' in text
    assert text.index('run_step "provider federation smoke unit"') < text.index('run_shell_step "provider federation smoke"')
    assert 'run_shell_step "provider federation smoke" "$ROOT_DIR/scripts/smoke-provider-federation.sh"' in text
    assert 'run_step "install server unit" python3 "$ROOT_DIR/scripts/test_install_server_unit.py"' in text
    assert text.index('run_step "install server unit"') < text.index('run_step "install layout smoke unit"')
    assert 'run_step "install layout smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_install_layout_unit.py"' in text
    assert text.index('run_step "install layout smoke unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "public ingress smoke unit" python3 "$ROOT_DIR/scripts/test_smoke_public_ingress_unit.py"' in text
    assert text.index('run_step "public ingress smoke unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "package release unit" python3 "$ROOT_DIR/scripts/test_package_release_unit.py"' in text
    assert text.index('run_step "package release unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "restart gateway unit" python3 "$ROOT_DIR/scripts/test_restart_gateway_unit.py"' in text
    assert text.index('run_step "restart gateway unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "rust production panic scan unit" python3 "$ROOT_DIR/scripts/test_rust_production_panic_scan_unit.py"' in text
    assert 'run_step "rust production panic scan" python3 "$ROOT_DIR/scripts/rust-production-panic-scan.py"' in text
    assert text.index('run_step "rust production panic scan unit"') < text.index('run_step "rust production panic scan"')
    assert text.index('run_step "rust production panic scan"') < text.index('echo "== release gate passed =="')
    assert 'run_step "web preview unit" python3 "$ROOT_DIR/scripts/test_start_web_preview_unit.py"' in text
    assert text.index('run_step "web preview unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "preview server unit" python3 "$ROOT_DIR/scripts/test_preview_server_unit.py"' in text
    assert text.index('run_step "preview server unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "device id unit" python3 "$ROOT_DIR/scripts/test_lobster_device_id_unit.py"' in text
    assert text.index('run_step "device id unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "web assets audit unit" python3 "$ROOT_DIR/scripts/test_audit_web_assets_unit.py"' in text
    assert text.index('run_step "web assets audit unit"') < text.index('echo "== release gate passed =="')
    assert 'run_step "complete verification unit" python3 "$ROOT_DIR/scripts/test_verify_complete_unit.py"' in text
    assert text.index('run_step "complete verification unit"') < text.index('echo "== release gate passed =="')
    assert 'export GATEWAY_BIN="' in text
    assert 'export CLI_BIN="' in text
    assert 'export TUI_BIN="' in text
    assert 'gateway_bin_default="$ROOT_DIR/target/debug/lobster-waku-gateway"' in text
    assert '${BIN_PATH:-' not in text, "release gate must not inherit provider-only BIN_PATH"
    assert 'export BIN_PATH=' not in text, "release gate must not leak provider-only BIN_PATH"
    assert text.index('export SKIP_BUILD=1') < text.index('run_step "makefile smoke unit"')
    assert "cargo build --quiet -p lobster-waku-gateway" in ci_text
    assert "node scripts/smoke-web-dual-browser.mjs" in ci_text
    return 0


if __name__ == "__main__":
    sys.exit(main())
