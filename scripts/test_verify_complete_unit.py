#!/usr/bin/env python3
from pathlib import Path
import os
import stat
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "verify-complete.sh"


def write_stub(bin_dir: Path, name: str, body: str) -> None:
    path = bin_dir / name
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def run_with_stubs(*, npm_status: int = 0, git_status: int = 0) -> tuple[int, str]:
    with tempfile.TemporaryDirectory(prefix="lobster-verify-complete.") as tmp:
        tmp_path = Path(tmp)
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        log_path = tmp_path / "verify.log"

        write_stub(
            bin_dir,
            "npm",
            f"#!/usr/bin/env bash\necho npm stub\nexit {npm_status}\n",
        )
        write_stub(
            bin_dir,
            "cargo",
            "#!/usr/bin/env bash\necho cargo stub: $*\nexit 0\n",
        )
        write_stub(
            bin_dir,
            "node",
            "#!/usr/bin/env bash\necho node stub: $*\nexit 0\n",
        )
        write_stub(
            bin_dir,
            "python3",
            "#!/usr/bin/env bash\necho python3 stub: $*\nexit 0\n",
        )
        write_stub(
            bin_dir,
            "git",
            f"#!/usr/bin/env bash\necho git stub\nexit {git_status}\n",
        )

        env = os.environ.copy()
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"
        env["LOG"] = str(log_path)
        result = subprocess.run(
            ["bash", str(SCRIPT)],
            cwd=str(ROOT),
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        return result.returncode, log_path.read_text(encoding="utf-8")


def main() -> int:
    assert SCRIPT.exists(), f"missing complete verification script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert "set -euo pipefail" in text
    assert 'ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"' in text
    assert 'LOG="${LOG:-$ROOT_DIR/verify-complete.log}"' in text
    assert "run_logged()" in text
    assert '"$@" 2>&1 | tee -a "$LOG"' in text
    assert 'local status="${PIPESTATUS[0]}"' in text
    assert 'if [[ "$status" -eq 0 ]]; then' in text
    assert 'echo "PASS: $label" | tee -a "$LOG"' in text
    assert 'echo "FAIL: $label" | tee -a "$LOG"' in text
    assert 'return "$status"' in text
    assert "overall_status=0" in text
    assert 'run_logged "frontend" npm test || overall_status=1' in text
    assert 'run_logged "gateway build" cargo build -p lobster-waku-gateway || overall_status=1' in text
    assert 'run_logged "gateway tests" cargo test -p lobster-waku-gateway || overall_status=1' in text
    assert 'run_logged "cli tests" cargo test -p lobster-cli || overall_status=1' in text
    assert 'run_logged "tui tests" cargo test -p lobster-tui || overall_status=1' in text
    assert 'run_logged "workspace tests" cargo test --workspace || overall_status=1' in text
    assert 'run_logged "native Waku REST adapter tests" cargo test -p transport-waku --features native-waku-rest || overall_status=1' in text
    assert 'run_logged "rust fmt" cargo fmt --check || overall_status=1' in text
    assert 'run_logged "rust lint" cargo clippy --workspace -- -D warnings || overall_status=1' in text
    assert 'run_logged "native Waku REST adapter lint" cargo clippy -p transport-waku --features native-waku-rest -- -D warnings || overall_status=1' in text
    assert text.index('run_logged "workspace tests"') < text.index('run_logged "native Waku REST adapter tests"')
    assert text.index('run_logged "native Waku REST adapter tests"') < text.index('run_logged "rust fmt"')
    assert text.index('run_logged "rust fmt"') < text.index('run_logged "rust lint"')
    assert text.index('run_logged "rust lint"') < text.index('run_logged "native Waku REST adapter lint"')
    assert 'run_logged "rust production panic scan unit" python3 "$ROOT_DIR/scripts/test_rust_production_panic_scan_unit.py" || overall_status=1' in text
    assert 'run_logged "rust production panic scan" python3 "$ROOT_DIR/scripts/rust-production-panic-scan.py" || overall_status=1' in text
    assert text.index('run_logged "rust production panic scan unit"') < text.index('run_logged "rust production panic scan"')
    assert 'run_logged "syntax: $f" node --check "$f" || overall_status=1' in text
    assert 'run_logged "workspace status" git status --short || overall_status=1' in text
    assert 'exit "$overall_status"' in text
    assert "if [ $? -eq 0 ]" not in text
    assert 'git status --short | tee -a "$LOG"' not in text

    rc, log = run_with_stubs(npm_status=7)
    assert rc == 1
    assert "FAIL: frontend" in log
    assert "PASS: gateway build" in log
    assert "PASS: workspace tests" in log
    assert "PASS: native Waku REST adapter tests" in log
    assert "PASS: rust fmt" in log
    assert "PASS: rust lint" in log
    assert "PASS: native Waku REST adapter lint" in log
    assert "PASS: rust production panic scan unit" in log
    assert "PASS: rust production panic scan" in log
    assert "PASS: workspace status" in log
    assert "DONE:" in log

    rc, log = run_with_stubs(git_status=9)
    assert rc == 1
    assert "PASS: frontend" in log
    assert "FAIL: workspace status" in log
    assert "DONE:" in log

    return 0


if __name__ == "__main__":
    sys.exit(main())
