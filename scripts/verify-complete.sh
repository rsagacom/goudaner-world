#!/usr/bin/env bash
# lobster-chat 完成性验证脚本
# 验证所有功能模块正常，结果输出到 verify-complete.log

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${LOG:-$ROOT_DIR/verify-complete.log}"
overall_status=0

: > "$LOG"

log() {
  echo "$@" | tee -a "$LOG"
}

section() {
  log ""
  log "=== $1 ==="
}

run_logged() {
  local label="$1"
  shift

  "$@" 2>&1 | tee -a "$LOG"
  local status="${PIPESTATUS[0]}"
  if [[ "$status" -eq 0 ]]; then
    echo "PASS: $label" | tee -a "$LOG"
  else
    echo "FAIL: $label" | tee -a "$LOG"
  fi
  return "$status"
}

log "=== lobster-chat Complete Verification ==="
date | tee -a "$LOG"

section "1. 前端测试"
cd "$ROOT_DIR/apps/lobster-web-shell"
run_logged "frontend" npm test || overall_status=1

section "2. Rust Gateway 编译"
cd "$ROOT_DIR"
run_logged "gateway build" cargo build -p lobster-waku-gateway || overall_status=1

section "3. Rust Gateway 测试"
run_logged "gateway tests" cargo test -p lobster-waku-gateway || overall_status=1

section "4. Rust CLI 测试"
run_logged "cli tests" cargo test -p lobster-cli || overall_status=1

section "5. Rust TUI 测试"
run_logged "tui tests" cargo test -p lobster-tui || overall_status=1

section "6. Rust Workspace 测试"
run_logged "workspace tests" cargo test --workspace || overall_status=1
run_logged "native Waku REST adapter tests" cargo test -p transport-waku --features native-waku-rest || overall_status=1

section "7. Rust 格式检查"
run_logged "rust fmt" cargo fmt --check || overall_status=1

section "8. Rust Lint"
run_logged "rust lint" cargo clippy --workspace -- -D warnings || overall_status=1
run_logged "native Waku REST adapter lint" cargo clippy -p transport-waku --features native-waku-rest -- -D warnings || overall_status=1

section "9. Rust 生产 panic 扫描"
run_logged "rust production panic scan unit" python3 "$ROOT_DIR/scripts/test_rust_production_panic_scan_unit.py" || overall_status=1
run_logged "rust production panic scan" python3 "$ROOT_DIR/scripts/rust-production-panic-scan.py" || overall_status=1

section "10. 语法检查"
cd "$ROOT_DIR/apps/lobster-web-shell"
for f in shell-*.js app.js; do
  run_logged "syntax: $f" node --check "$f" || overall_status=1
done

section "11. 工作区状态"
cd "$ROOT_DIR"
run_logged "workspace status" git status --short || overall_status=1

log "DONE: $(date)"
exit "$overall_status"
