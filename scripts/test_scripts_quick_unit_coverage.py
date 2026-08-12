#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = ROOT / "scripts"
RELEASE_GATE = SCRIPTS_DIR / "smoke-release-gate.sh"

SCRIPT_UNIT_MAP = {
    "audit-web-assets.sh": "test_audit_web_assets_unit.py",
    "backup-state.sh": "test_backup_state_unit.py",
    "install-server.sh": "test_install_server_unit.py",
    "lobster-device-id.sh": "test_lobster_device_id_unit.py",
    "package-release.sh": "test_package_release_unit.py",
    "production-readiness.sh": "test_production_readiness_unit.py",
    "preflight.sh": "test_preflight_unit.py",
    "preview-server.mjs": "test_preview_server_unit.py",
    "restart-gateway.sh": "test_restart_gateway_unit.py",
    "rust-production-panic-scan.py": "test_rust_production_panic_scan_unit.py",
    "smoke-auth-registration.sh": "test_smoke_auth_registration_unit.py",
    "smoke-cli-channel.sh": "test_smoke_cli_channel_unit.py",
    "smoke-install-layout.sh": "test_smoke_install_layout_unit.py",
    "smoke-provider-federation.sh": "test_smoke_provider_federation_unit.py",
    "smoke-public-ingress.sh": "test_smoke_public_ingress_unit.py",
    "smoke-release-gate.sh": "test_smoke_release_gate_unit.py",
    "smoke-resident-mainline.sh": "test_smoke_resident_mainline_unit.py",
    "smoke-shell-direct-http.sh": "test_smoke_shell_direct_http_unit.py",
    "smoke-shell-dual-http.sh": "test_smoke_shell_dual_http_unit.py",
    "smoke-web-dual-browser.mjs": "test_smoke_web_dual_browser_unit.py",
    "smoke-web-shell.sh": "test_smoke_web_shell_unit.py",
    "start-terminal.sh": "test_start_terminal_shell_unit.py",
    "start-web-preview.sh": "test_start_web_preview_unit.py",
    "verify-complete.sh": "test_verify_complete_unit.py",
}


def main() -> int:
    missing_scripts = [
        script for script in SCRIPT_UNIT_MAP if not (SCRIPTS_DIR / script).exists()
    ]
    assert not missing_scripts, f"missing scripts: {missing_scripts}"

    missing_units = [
        unit for unit in SCRIPT_UNIT_MAP.values() if not (SCRIPTS_DIR / unit).exists()
    ]
    assert not missing_units, f"missing quick unit tests: {missing_units}"

    release_gate_text = RELEASE_GATE.read_text(encoding="utf-8")
    release_gate_units = {
        "test_scripts_quick_unit_coverage.py",
        "test_smoke_runtime_contract_unit.py",
        "test_preflight_unit.py",
        "test_makefile_unit.py",
        "test_smoke_cli_channel_unit.py",
        "test_smoke_auth_registration_unit.py",
        "test_smoke_resident_mainline_unit.py",
        "test_smoke_shell_dual_http_unit.py",
        "test_smoke_shell_direct_http_unit.py",
        "test_smoke_web_shell_unit.py",
        "test_smoke_web_dual_browser_unit.py",
        "test_start_terminal_unit.py",
        "test_start_terminal_shell_unit.py",
        "test_smoke_provider_federation_unit.py",
        "test_install_server_unit.py",
        "test_smoke_install_layout_unit.py",
        "test_smoke_public_ingress_unit.py",
        "test_package_release_unit.py",
        "test_production_readiness_unit.py",
        "test_release_workflow_unit.py",
        "test_restart_gateway_unit.py",
        "test_rust_production_panic_scan_unit.py",
        "test_start_web_preview_unit.py",
        "test_preview_server_unit.py",
        "test_lobster_device_id_unit.py",
        "test_audit_web_assets_unit.py",
        "test_backup_state_unit.py",
        "test_verify_complete_unit.py",
    }
    missing_from_gate = [
        unit for unit in sorted(release_gate_units) if unit not in release_gate_text
    ]
    assert not missing_from_gate, f"release gate missing quick units: {missing_from_gate}"

    return 0


if __name__ == "__main__":
    sys.exit(main())
