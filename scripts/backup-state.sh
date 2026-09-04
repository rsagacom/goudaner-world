#!/usr/bin/env bash
# lobster-chat 状态目录备份脚本（生产定期备份）
#
# 固化 docs/DEPLOYMENT.md §8 的手动流程，并补上 2026-08-01/08-02 演练教训：
#   - 备份前停 Gateway 保证多 JSON 一致时间点，备完立即拉起（秒级停机）
#   - 备份后校验：tarball 非空、包含 timelines/、解压测试通过，失败即报警保留现场
#   - 轮转保留最近 KEEP 份，防止 /srv/backups 无限增长
#
# 用法: sudo bash scripts/backup-state.sh
# 定期化: 配合 systemd timer（见 docs/DEPLOYMENT.md §8）
set -euo pipefail

STATE_DIR="${STATE_DIR:-/var/lib/lobster-chat}"
BACKUP_DIR="${BACKUP_DIR:-/srv/backups}"
SERVICE="${SERVICE:-lobster-waku-gateway}"
KEEP="${KEEP:-14}"

TS="$(date +%Y%m%d-%H%M%S)"
ARCHIVE="$BACKUP_DIR/lobster-chat-state-$TS.tar.gz"

log() { echo "[backup-state] $*"; }
fail() { echo "[backup-state] ERROR: $*" >&2; exit 1; }

[[ -d "$STATE_DIR" ]] || fail "state dir not found: $STATE_DIR"
mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR" 2>/dev/null || true

# 磁盘水位检查：状态目录很小（KB~MB 级），但 /srv 所在盘低于 1G 时不写
avail_kb=$(df -k "$BACKUP_DIR" | awk 'NR==2 {print $4}')
[[ "$avail_kb" -gt 1048576 ]] || fail "low disk space on $BACKUP_DIR (${avail_kb}KB avail)"

was_active=0
if systemctl is-active --quiet "$SERVICE"; then
  was_active=1
  log "stopping $SERVICE for consistent snapshot"
  systemctl stop "$SERVICE"
fi

# 无论 tar 成败都要把服务拉回来
restore_service() {
  if [[ "$was_active" == "1" ]]; then
    systemctl start "$SERVICE" || true
    log "$SERVICE restarted"
  fi
}
trap restore_service EXIT

log "archiving $STATE_DIR -> $ARCHIVE"
tar -C "$(dirname "$STATE_DIR")" -czf "$ARCHIVE" "$(basename "$STATE_DIR")" \
  || fail "tar failed"

restore_service
trap - EXIT

# 校验：非空、含 timelines 目录、可完整解压。先完整读取目录清单再 grep，
# 避免 set -o pipefail 下 grep -q 提前退出导致 tar 收到 SIGPIPE 并误报失败。
[[ -s "$ARCHIVE" ]] || fail "archive is empty: $ARCHIVE"
archive_listing="$(tar -tzf "$ARCHIVE")" || fail "archive failed listing: $ARCHIVE"
grep -q "timelines/" <<<"$archive_listing" || fail "archive missing timelines/: $ARCHIVE"
# 附件目录（图片消息资产）：存在即必须在档
if [[ -d "$STATE_DIR/attachments" ]]; then
  grep -q "attachments/" <<<"$archive_listing" || fail "archive missing attachments/: $ARCHIVE"
fi
tar -tzf "$ARCHIVE" > /dev/null || fail "archive failed integrity read: $ARCHIVE"

# 关键状态文件在档校验（2026-09-05 WebPush）：存在即必须在档，防止存储布局
# 变更后备份静默缺失（push 订阅丢失=用户全部重订；VAPID 私钥丢失=所有
# 浏览器订阅失效必须重新授权）。
for critical in auth-state.json push-subscriptions.json vapid-signing-key.json; do
  if [[ -f "$STATE_DIR/$critical" ]]; then
    grep -q "$critical" <<<"$archive_listing" \
      || fail "archive missing critical state file: $critical"
  fi
done
size="$(du -h "$ARCHIVE" | cut -f1)"
log "archive verified ($size)"

# 轮转：按时间序保留最近 KEEP 份
cd "$BACKUP_DIR"
# shellcheck disable=SC2012
ls -1t lobster-chat-state-*.tar.gz 2>/dev/null | tail -n +"$((KEEP + 1))" | while read -r old; do
  log "rotating out $old"
  rm -f "$old"
done

log "done: $ARCHIVE"
