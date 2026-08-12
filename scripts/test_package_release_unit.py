#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "package-release.sh"


def main() -> int:
    assert SCRIPT.exists(), f"missing package release script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert 'DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist}"' in text
    assert 'SKIP_BUILD="${SKIP_BUILD:-0}"' in text
    assert 'HOST_TARGET_OVERRIDE="${HOST_TARGET_OVERRIDE:-}"' in text
    assert 'GATEWAY_BINARY_PATH="${GATEWAY_BINARY_PATH:-$ROOT_DIR/target/release/lobster-waku-gateway}"' in text
    assert 'ALLOW_DIRTY_RELEASE="${ALLOW_DIRTY_RELEASE:-0}"' in text
    assert 'RELEASE_GIT_SHA="${RELEASE_GIT_SHA:-}"' in text
    assert 'git -C "$ROOT_DIR" status --porcelain --untracked-files=normal' in text
    assert "refusing release package from dirty worktree" in text
    assert 'git -C "$ROOT_DIR" rev-parse HEAD' in text
    assert "sha256_file()" in text
    assert "need_cmd tar" in text
    assert 'if [[ -z "$HOST_TARGET_OVERRIDE" ]]; then' in text
    assert text.index('if [[ -z "$HOST_TARGET_OVERRIDE" ]]; then') < text.index("need_cmd rustc")
    assert text.index("need_cmd rustc") < text.index("host_target=\"$(rustc -vV | awk '/host:/ { print $2 }')\"")
    assert 'else\n  host_target="$HOST_TARGET_OVERRIDE"\nfi' in text
    assert text.index('if [[ "$SKIP_BUILD" != "1" ]]; then') < text.index("need_cmd cargo")
    assert text.index("need_cmd cargo") < text.index('cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway')
    assert 'mkdir -p "$DIST_DIR"' in text
    assert "host_target=\"$(rustc -vV | awk '/host:/ { print $2 }')\"" in text
    assert 'bin_name="lobster-waku-gateway-${host_target}"' in text
    assert 'binary_path="$GATEWAY_BINARY_PATH"' in text
    assert 'cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --release -p lobster-waku-gateway' in text
    assert '--exclude="$(basename "$ROOT_DIR")/.git"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/.playwright-cli"' in text
    assert '--exclude=".DS_Store"' in text
    assert '--exclude="*/.DS_Store"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/node_modules"' in text
    assert '--exclude="*/node_modules"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/target"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/dist"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/.lobster-chat-dev"' in text
    assert '--exclude="$(basename "$ROOT_DIR")/backups"' in text
    assert '--exclude="*/backups"' in text
    assert '--exclude="*/test-results"' in text
    assert '--exclude="*/screenshots"' in text
    assert '--exclude="*/.tmp"' in text
    assert '--exclude="*.source.html"' in text
    assert '--exclude="*/*.source.html"' in text
    assert '-czf "$DIST_DIR/lobster-chat-source.tar.gz"' in text
    assert '-C "$(dirname "$ROOT_DIR")"' in text
    assert '"$(basename "$ROOT_DIR")"' in text
    assert 'tar \\\n  --exclude="./node_modules" \\\n  --exclude="./backups" \\\n  --exclude="./.tmp" \\\n  --exclude="./test" \\\n  --exclude="./test-results" \\\n  --exclude="./screenshots" \\\n  --exclude="./*.mjs" \\\n  --exclude="./.DS_Store" \\\n  --exclude="*/.DS_Store" \\\n  --exclude="*.source.html" \\\n  --exclude="*/*.source.html" \\\n  -czf "$DIST_DIR/lobster-web-shell.tar.gz" \\\n  -C "$ROOT_DIR/apps/lobster-web-shell" \\\n  .' in text
    assert 'if [[ -x "$binary_path" ]]; then' in text
    assert 'tar -czf "$DIST_DIR/${bin_name}.tar.gz" -C "$(dirname "$binary_path")" lobster-waku-gateway' in text
    assert 'warning: release gateway binary not found at $binary_path' in text
    assert '> "$DIST_DIR/release-manifest.json"' in text
    assert '"schema_version\\":1' in text
    assert '"git_sha\\":\\"$RELEASE_GIT_SHA' in text
    assert '"sha256\\":\\"$source_sha' in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
