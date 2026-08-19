#!/usr/bin/env python3
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"


def main() -> int:
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "workflow_dispatch:" in text
    assert "runs-on: ubuntu-latest" in text
    assert "runner: ubuntu-24.04-arm" in text
    assert "machine: x86_64" in text
    assert "machine: aarch64" in text
    assert "cargo test --workspace --quiet" in text
    assert "cargo test -p transport-waku --features native-waku-rest --quiet" in text
    assert "npm test" in text
    assert "HOST_TARGET_OVERRIDE: ${{ matrix.target }}" in text
    assert "GATEWAY_BINARY_PATH: ${{ github.workspace }}/target/${{ matrix.target }}/release/lobster-waku-gateway" in text
    assert 'cargo build --release --target "${{ matrix.target }}" -p lobster-waku-gateway' in text
    assert "LOBSTER_BUILD_GIT_SHA: ${{ github.sha }}" in text
    assert "RELEASE_GIT_SHA: ${{ github.sha }}" in text
    assert 'test "$(uname -m)" = "${{ matrix.machine }}"' in text
    assert 'test "$(rustc -vV | awk \'/host:/ { print $2 }\')" = "${{ matrix.target }}"' in text
    assert "target: aarch64-unknown-linux-gnu" in text
    assert "target: x86_64-unknown-linux-gnu" in text
    assert "DIST_DIR: ${{ runner.temp }}/lobster-dist" in text
    assert 'SKIP_BUILD: "1"' in text
    assert "lobster-chat-release-${{ github.sha }}-${{ matrix.target }}" in text
    assert "actions/upload-artifact@v4" in text
    assert "lobster-waku-gateway-${{ matrix.target }}.tar.gz" in text
    assert "lobster-web-shell.tar.gz" in text
    assert "lobster-chat-source.tar.gz" in text
    assert "release-manifest.json" in text
    assert "jq -r .git_sha" in text
    assert '(cd "$RUNNER_TEMP/lobster-dist" && sha256sum *.tar.gz) | tee "$RUNNER_TEMP/lobster-dist/SHA256SUMS"' in text
    assert "LOBSTER_DEV_EMAIL_OTP_INLINE" not in text
    assert "LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN" not in text
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
