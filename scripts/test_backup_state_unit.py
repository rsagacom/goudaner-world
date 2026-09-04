#!/usr/bin/env python3
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "backup-state.sh"


class BackupStateTests(unittest.TestCase):
    def test_backup_succeeds_without_pipefail_sigpipe(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            state_dir = root / "state"
            backup_dir = root / "backups"
            (state_dir / "timelines").mkdir(parents=True)
            (state_dir / "timelines" / "room.json").write_text("{}", encoding="utf-8")
            backup_dir.mkdir()

            env = os.environ.copy()
            env.update(
                STATE_DIR=str(state_dir),
                BACKUP_DIR=str(backup_dir),
                SERVICE="lobster-backup-unit-nonexistent",
                KEEP="2",
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )

            archives = list(backup_dir.glob("lobster-chat-state-*.tar.gz"))
            self.assertEqual(len(archives), 1)
            self.assertIn("archive verified", completed.stdout)
            listing = subprocess.run(
                ["tar", "-tzf", str(archives[0])],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            self.assertIn("timelines/room.json", listing)

    def test_backup_covers_webpush_state_files(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            state_dir = root / "state"
            backup_dir = root / "backups"
            (state_dir / "timelines").mkdir(parents=True)
            (state_dir / "timelines" / "room.json").write_text("{}", encoding="utf-8")
            # WebPush 状态（2026-09-05）：订阅与 VAPID 私钥必须入档
            (state_dir / "push-subscriptions.json").write_text("[]", encoding="utf-8")
            (state_dir / "vapid-signing-key.json").write_text('{"pkcs8": []}', encoding="utf-8")
            # 附件目录（图片消息资产）存在即必须在档
            (state_dir / "attachments").mkdir()
            (state_dir / "attachments" / "a.png").write_bytes(b"png")
            backup_dir.mkdir()

            env = os.environ.copy()
            env.update(
                STATE_DIR=str(state_dir),
                BACKUP_DIR=str(backup_dir),
                SERVICE="lobster-backup-unit-nonexistent",
                KEEP="2",
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertIn("archive verified", completed.stdout)
            archives = list(backup_dir.glob("lobster-chat-state-*.tar.gz"))
            self.assertEqual(len(archives), 1)
            listing = subprocess.run(
                ["tar", "-tzf", str(archives[0])],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            self.assertIn("attachments/a.png", listing)

    def test_backup_fails_closed_when_critical_file_missing_from_archive(self):
        # PATH 注入 tar 包装器：打包瞬间把 push-subscriptions.json 移走，
        # 使生成的档案缺失关键文件——脚本必须 fail-closed 报错而不是静默通过。
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            state_dir = root / "state"
            backup_dir = root / "backups"
            fake_bin = root / "fakebin"
            (state_dir / "timelines").mkdir(parents=True)
            fake_bin.mkdir()
            backup_dir.mkdir()
            (state_dir / "timelines" / "room.json").write_text("{}", encoding="utf-8")
            (state_dir / "push-subscriptions.json").write_text("[]", encoding="utf-8")
            (state_dir / "vapid-signing-key.json").write_text("{}", encoding="utf-8")

            stash = root / "stash"
            stash.mkdir()
            wrapper = fake_bin / "tar"
            wrapper.write_text(
                "#!/usr/bin/env bash\n"
                "create=false\n"
                'for a in "$@"; do [ "$a" = "-czf" ] && create=true; done\n'
                "if $create; then\n"
                '  mv "$STATE_DIR/push-subscriptions.json" "%s/push-subscriptions.json.bak"\n'
                "fi\n"
                '  /usr/bin/tar "$@"\n'
                "if $create; then\n"
                '  mv "%s/push-subscriptions.json.bak" "$STATE_DIR/push-subscriptions.json"\n'
                "fi\n" % (stash, stash),
                encoding="utf-8",
            )
            wrapper.chmod(0o755)

            env = os.environ.copy()
            env.update(
                STATE_DIR=str(state_dir),
                BACKUP_DIR=str(backup_dir),
                SERVICE="lobster-backup-unit-nonexistent",
                KEEP="2",
                PATH=f"{fake_bin}{os.pathsep}{env.get('PATH', '')}",
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn(
                "archive missing critical state file: push-subscriptions.json",
                completed.stderr,
            )
            # 关键文件必须被还原到 STATE_DIR（包装器职责），防止备份事故放大
            self.assertTrue((state_dir / "push-subscriptions.json").exists())

    def test_backup_skips_optional_files_when_absent(self):
        # 关键文件尚未生成（未启用推送）时脚本不得误报
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            state_dir = root / "state"
            backup_dir = root / "backups"
            (state_dir / "timelines").mkdir(parents=True)
            (state_dir / "timelines" / "room.json").write_text("{}", encoding="utf-8")
            backup_dir.mkdir()

            env = os.environ.copy()
            env.update(
                STATE_DIR=str(state_dir),
                BACKUP_DIR=str(backup_dir),
                SERVICE="lobster-backup-unit-nonexistent",
                KEEP="2",
            )
            completed = subprocess.run(
                ["bash", str(SCRIPT)],
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertIn("archive verified", completed.stdout)

if __name__ == "__main__":
    unittest.main()
