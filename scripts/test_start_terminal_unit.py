#!/usr/bin/env python3
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).with_name("test_start_terminal.py")


def load_target_module():
    spec = importlib.util.spec_from_file_location("test_start_terminal", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ResolveRootTests(unittest.TestCase):
    def test_resolve_root_defaults_to_repo_root_from_script_path(self):
        target = load_target_module()
        expected = str(SCRIPT_PATH.resolve().parent.parent)
        self.assertEqual(target.resolve_root(), expected)

    def test_resolve_root_prefers_environment_override(self):
        target = load_target_module()
        with tempfile.TemporaryDirectory() as temp_dir:
            with mock.patch.dict(target.os.environ, {"LOBSTER_CHAT_ROOT": temp_dir}, clear=False):
                self.assertEqual(target.resolve_root(), temp_dir)


class SmokeDumpTests(unittest.TestCase):
    def test_run_smoke_json_requests_json_dump_and_parses_payload(self):
        target = load_target_module()
        target.WEB_GENERATED_DIR = "/tmp/lobster-web-generated"
        completed = mock.Mock(stdout='{"surface_kind":"CityPublic","visible_panels":["status"]}')

        with mock.patch.object(target.subprocess, "run", return_value=completed) as run_mock:
            payload = target.run_smoke_json("user", "/tmp/lobster-state")

        self.assertEqual(payload["surface_kind"], "CityPublic")
        _, kwargs = run_mock.call_args
        self.assertEqual(kwargs["env"]["LOBSTER_TUI_SMOKE_DUMP"], "json")
        self.assertEqual(kwargs["env"]["LOBSTER_TUI_STATE_DIR"], "/tmp/lobster-state")
        self.assertEqual(
            kwargs["env"]["LOBSTER_WEB_GENERATED_DIR"],
            "/tmp/lobster-web-generated",
        )


class HealthProbeTests(unittest.TestCase):
    def test_wait_for_health_bypasses_local_http_proxy(self):
        target = load_target_module()
        completed = mock.Mock(returncode=0)

        with mock.patch.object(target.subprocess, "run", return_value=completed) as run_mock:
            target.wait_for_health("http://127.0.0.1:8798")

        _, kwargs = run_mock.call_args
        self.assertIn("127.0.0.1", kwargs["env"]["NO_PROXY"])
        self.assertIn("localhost", kwargs["env"]["NO_PROXY"])
        self.assertIn("127.0.0.1", kwargs["env"]["no_proxy"])
        self.assertIn("localhost", kwargs["env"]["no_proxy"])


class CliScopedReadAuthTests(unittest.TestCase):
    def test_cli_scoped_reads_forward_resident_session_token(self):
        target = load_target_module()
        target.SESSION_TOKENS = {"rsaga": "session-rsaga"}
        completed = mock.Mock(stdout='{"messages":[]}')

        with mock.patch.object(target.subprocess, "run", return_value=completed) as run_mock:
            target.rooms_json("user:rsaga")
            target.tail_json("user:rsaga", "room:world:lobby")

        rooms_args = run_mock.call_args_list[0].args[0]
        tail_args = run_mock.call_args_list[1].args[0]
        self.assertIn("--token", rooms_args)
        self.assertIn("session-rsaga", rooms_args)
        self.assertIn("--token", tail_args)
        self.assertIn("session-rsaga", tail_args)


class SessionBootstrapTests(unittest.TestCase):
    def test_issue_session_token_uses_local_no_proxy_opener(self):
        target = load_target_module()
        target.GATEWAY_URL = "http://127.0.0.1:8798"
        target.LOCAL_HTTP_OPENER = mock.Mock()

        def response(payload):
            context = mock.MagicMock()
            context.__enter__.return_value = io.BytesIO(
                json.dumps(payload).encode("utf-8")
            )
            return context

        target.LOCAL_HTTP_OPENER.open.side_effect = [
            response({"challenge_id": "otp:test", "dev_code": "123456"}),
            response({"session_token": "session-rsaga"}),
        ]

        token = target.issue_session_token("rsaga")

        self.assertEqual(token, "session-rsaga")
        self.assertEqual(target.LOCAL_HTTP_OPENER.open.call_count, 2)


if __name__ == "__main__":
    unittest.main()
