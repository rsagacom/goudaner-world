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


if __name__ == "__main__":
    unittest.main()
