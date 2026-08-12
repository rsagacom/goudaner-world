#!/usr/bin/env python3
import json
import os
import pty
import shutil
import signal
import select
import socket
import subprocess
import sys
import tempfile
import time
import threading
import urllib.request


def resolve_root() -> str:
    override = os.environ.get("LOBSTER_CHAT_ROOT")
    if override:
        return override
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


ROOT = resolve_root()
HOST = "127.0.0.1"
PORT = "8798"
GATEWAY_URL = f"http://{HOST}:{PORT}"
SKIP_BUILD = os.environ.get("LOBSTER_CHAT_SKIP_BUILD") == "1" or os.environ.get("SKIP_BUILD") == "1"
GATEWAY_BIN = os.environ.get("GATEWAY_BIN", f"{ROOT}/target/debug/lobster-waku-gateway")
CLI_BIN = os.environ.get("CLI_BIN", f"{ROOT}/target/debug/lobster-cli")
TUI_BIN = os.environ.get("TUI_BIN", f"{ROOT}/target/debug/lobster-tui")
WEB_GENERATED_DIR = os.environ.get("LOBSTER_WEB_GENERATED_DIR")
SEEDED_TEXT = "TUI_SMOKE_预置消息"
DIRECT_TEXT = "DIRECT_SMOKE_探针消息"
DIRECT_PEER_ID = "guide"
DIRECT_RESIDENT_ID = "tiyan"
DIRECT_CONVERSATION_ID = "dm:guide:tiyan"
USER_SEEDED_TEXT = "USER_SMOKE_城邦大厅探针消息"
ADMIN_SEEDED_TEXT = "ADMIN_SMOKE_城务告示探针消息"
WORLD_TUI_SUBMIT_TEXT = "WORLD_TUI_SEND_SMOKE_探针消息"
RUNNING_WORLD_RECOVERY_TEXT = "WORLD_RUNNING_SESSION_RECOVERY_探针消息"
RUNNING_WORLD_OFFLINE_SEND_TEXT = "WORLD_RUNNING_SESSION_OFFLINE_SEND_探针消息"
TUI_SUBMIT_TEXT = "TUI_SEND_SMOKE_探针消息"
USER_TUI_SUBMIT_TEXT = "USER_TUI_SEND_SMOKE_探针消息"
ADMIN_TUI_SUBMIT_TEXT = "ADMIN_TUI_SEND_SMOKE_探针消息"
USER_WORLD_COMMAND_TEXT = "USER_WORLD_COMMAND_探针消息"
USER_DIRECT_COMMAND_TEXT = "USER_DIRECT_COMMAND_探针消息"
USER_DM_COMMAND_TEXT = "USER_DM_COMMAND_探针消息"
ADMIN_WORLD_COMMAND_TEXT = "ADMIN_WORLD_COMMAND_探针消息"
ADMIN_GOVERNANCE_COMMAND_TEXT = "ADMIN_GOVERNANCE_COMMAND_探针消息"
USE_DEFAULT_GATEWAY = object()
SESSION_TOKENS: dict[str, str] = {}
LOCAL_HTTP_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def local_probe_env() -> dict[str, str]:
    env = os.environ.copy()
    for key in ("NO_PROXY", "no_proxy"):
        values = [value for value in env.get(key, "").split(",") if value]
        for host in ("127.0.0.1", "localhost"):
            if host not in values:
                values.append(host)
        env[key] = ",".join(values)
    return env


def isolate_tui_generated_output(env: dict[str, str]) -> None:
    if WEB_GENERATED_DIR:
        env["LOBSTER_WEB_GENERATED_DIR"] = WEB_GENERATED_DIR


def fail(label: str, payload: str) -> None:
    print(f"FAIL::{label}")
    print(payload)
    raise SystemExit(1)


def ensure_binaries() -> None:
    if SKIP_BUILD:
        return
    subprocess.run(
        ["cargo", "build", "-p", "lobster-waku-gateway", "-p", "lobster-cli", "-p", "lobster-tui"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def wait_for_health(url: str) -> None:
    deadline = time.time() + 20
    while time.time() < deadline:
        probe = subprocess.run(
            ["curl", "-fsS", f"{url}/health"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=local_probe_env(),
        )
        if probe.returncode == 0:
            return
        time.sleep(0.25)
    raise TimeoutError(f"gateway did not become healthy: {url}")


def reserve_port() -> str:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((HOST, 0))
        return str(sock.getsockname()[1])


def send_room_message(text: str) -> None:
    subprocess.run(
        [
            CLI_BIN,
            "send",
            "--from",
            "agent:openclaw",
            "--to",
            "room:world:lobby",
            "--text",
            text,
            "--gateway",
            GATEWAY_URL,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def seed_room_message() -> None:
    send_room_message(SEEDED_TEXT)


def send_city_message(text: str) -> None:
    subprocess.run(
        [
            CLI_BIN,
            "send",
            "--from",
            "user:builder",
            "--to",
            "room:city:core-harbor:lobby",
            "--text",
            text,
            "--gateway",
            GATEWAY_URL,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def seed_city_message() -> None:
    send_city_message(USER_SEEDED_TEXT)


def send_governance_message(text: str) -> None:
    subprocess.run(
        [
            CLI_BIN,
            "send",
            "--from",
            "user:builder",
            "--to",
            "room:city:aurora-hub:announcements",
            "--text",
            text,
            "--gateway",
            GATEWAY_URL,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def seed_governance_message() -> None:
    send_governance_message(ADMIN_SEEDED_TEXT)


def seed_direct_message() -> None:
    subprocess.run(
        [
            CLI_BIN,
            "send",
            "--from",
            f"user:{DIRECT_PEER_ID}",
            "--to",
            f"user:{DIRECT_RESIDENT_ID}",
            "--text",
            DIRECT_TEXT,
            "--gateway",
            GATEWAY_URL,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def issue_session_token(resident_id: str) -> str:
    payload = json.dumps(
        {"email": f"terminal.{resident_id}@example.com", "resident_id": resident_id}
    ).encode("utf-8")
    request = urllib.request.Request(
        f"{GATEWAY_URL}/v1/auth/email-otp/request",
        data=payload,
        headers={"content-type": "application/json"},
        method="POST",
    )
    with LOCAL_HTTP_OPENER.open(request, timeout=5.0) as response:
        challenge = json.loads(response.read().decode("utf-8"))
    verify_payload = json.dumps(
        {
            "challenge_id": challenge["challenge_id"],
            "code": challenge["dev_code"],
            "resident_id": resident_id,
        }
    ).encode("utf-8")
    verify_request = urllib.request.Request(
        f"{GATEWAY_URL}/v1/auth/email-otp/verify",
        data=verify_payload,
        headers={"content-type": "application/json"},
        method="POST",
    )
    with LOCAL_HTTP_OPENER.open(verify_request, timeout=5.0) as response:
        verified = json.loads(response.read().decode("utf-8"))
    return verified["session_token"]


def session_token_for_identity(identity: str) -> str:
    prefix, _, resident_id = identity.partition(":")
    if prefix != "user" or not resident_id:
        raise ValueError(f"scoped CLI reads require a user identity: {identity}")
    try:
        return SESSION_TOKENS[resident_id]
    except KeyError as exc:
        raise RuntimeError(f"missing terminal smoke session for {identity}") from exc


def rooms_json(identity: str) -> str:
    proc = subprocess.run(
        [
            CLI_BIN,
            "rooms",
            "--for",
            identity,
            "--token",
            session_token_for_identity(identity),
            "--gateway",
            GATEWAY_URL,
            "--json",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        timeout=12.0,
    )
    return proc.stdout


def tail_json(identity: str, conversation_id: str) -> dict:
    proc = subprocess.run(
        [
            CLI_BIN,
            "tail",
            "--for",
            identity,
            "--conversation-id",
            conversation_id,
            "--token",
            session_token_for_identity(identity),
            "--gateway",
            GATEWAY_URL,
            "--json",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        timeout=12.0,
    )
    return json.loads(proc.stdout)


def wait_for_tail_message(identity: str, conversation_id: str, text: str) -> dict:
    for _ in range(40):
        payload = tail_json(identity, conversation_id)
        if any(message["text"] == text for message in payload["messages"]):
            return payload
        time.sleep(0.25)
    return tail_json(identity, conversation_id)


def wait_for_dump_contains(
    mode: str,
    state_dir: str,
    required_markers: list[str],
    required_texts: list[str],
    timeout_secs: float = 12.0,
    gateway_url: str | None | object = USE_DEFAULT_GATEWAY,
) -> str:
    deadline = time.time() + timeout_secs
    last_output = ""
    while time.time() < deadline:
        last_output = run_smoke(
            mode,
            state_dir,
            gateway_url=gateway_url,
        )
        if all(marker in last_output for marker in required_markers) and all(
            text in last_output for text in required_texts
        ):
            return last_output
        time.sleep(0.25)
    return last_output


def assert_snapshot_contract(
    label: str,
    snapshot: dict,
    *,
    expected_surface: str,
    expected_active: str | None,
    required_panels: list[str],
    forbidden_panels: list[str] | None = None,
) -> None:
    if snapshot.get("surface_kind") != expected_surface:
        fail(label, json.dumps(snapshot, ensure_ascii=False, indent=2))
    if expected_active is not None and snapshot.get("active_conversation_id") != expected_active:
        fail(label, json.dumps(snapshot, ensure_ascii=False, indent=2))
    panels = snapshot.get("visible_panels") or []
    if not all(panel in panels for panel in required_panels):
        fail(label, json.dumps(snapshot, ensure_ascii=False, indent=2))
    if forbidden_panels and any(panel in panels for panel in forbidden_panels):
        fail(label, json.dumps(snapshot, ensure_ascii=False, indent=2))


def run_smoke(
    mode: str,
    state_dir: str,
    timeout_secs: float = 12.0,
    gateway_url: str | None | object = USE_DEFAULT_GATEWAY,
    dump_format: str = "plain",
) -> str:
    if gateway_url is USE_DEFAULT_GATEWAY:
        gateway_url = GATEWAY_URL
    env = os.environ.copy()
    env["TERM"] = "xterm-256color"
    if gateway_url is not None:
        env["LOBSTER_WAKU_GATEWAY_URL"] = gateway_url
    else:
        env.pop("LOBSTER_WAKU_GATEWAY_URL", None)
    env["LOBSTER_TUI_SMOKE_DUMP"] = dump_format
    env["LOBSTER_TUI_STATE_DIR"] = state_dir
    isolate_tui_generated_output(env)
    proc = subprocess.run(
        [TUI_BIN, "--mode", mode],
        cwd=ROOT,
        env=env,
        check=True,
        capture_output=True,
        text=True,
        timeout=timeout_secs,
    )
    return proc.stdout


def run_smoke_json(
    mode: str,
    state_dir: str,
    timeout_secs: float = 12.0,
    gateway_url: str | None | object = USE_DEFAULT_GATEWAY,
) -> dict:
    return json.loads(
        run_smoke(
            mode,
            state_dir,
            timeout_secs=timeout_secs,
            gateway_url=gateway_url,
            dump_format="json",
        )
    )


class LiveTuiSession:
    def __init__(self, mode: str, state_dir: str) -> None:
        self._stop = threading.Event()
        env = os.environ.copy()
        env["TERM"] = "xterm-256color"
        env["LOBSTER_WAKU_GATEWAY_URL"] = GATEWAY_URL
        env["LOBSTER_TUI_STATE_DIR"] = state_dir
        isolate_tui_generated_output(env)
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(ROOT)
            os.execve(
                TUI_BIN,
                [TUI_BIN, "--mode", mode],
                env,
            )
        self.pid = pid
        self.fd = fd
        self._drainer = threading.Thread(target=self._drain_output, daemon=True)
        self._drainer.start()

    def _drain_output(self) -> None:
        while not self._stop.is_set():
            try:
                ready, _, _ = select.select([self.fd], [], [], 0.1)
            except OSError:
                return
            if self.fd in ready:
                try:
                    os.read(self.fd, 4096)
                except OSError:
                    return

    def assert_alive(self, label: str) -> None:
        waited = os.waitpid(self.pid, os.WNOHANG)
        if waited != (0, 0):
            fail(label, f"running TUI exited unexpectedly: {waited}")

    def submit(self, texts: list[str]) -> None:
        os.write(self.fd, b"i")
        time.sleep(0.3)
        for text in texts:
            os.write(self.fd, text.encode("utf-8"))
            time.sleep(0.3)
            os.write(self.fd, b"\r")
            time.sleep(0.5)

    def close(self) -> None:
        self._stop.set()
        self._drainer.join(timeout=1.0)
        try:
            os.close(self.fd)
        except OSError:
            pass
        try:
            os.kill(self.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            os.waitpid(self.pid, 0)
        except (ChildProcessError, OSError):
            pass


def start_gateway(state_root: str) -> subprocess.Popen[str]:
    env = os.environ.copy()
    # This smoke uses synthetic agent/user identities; keep the dev bypass and inline OTP explicit.
    env["LOBSTER_DEV_AUTH_BYPASS"] = "1"
    env["LOBSTER_DEV_EMAIL_OTP_INLINE"] = "1"
    gateway_proc = subprocess.Popen(
        [
            GATEWAY_BIN,
            "--host",
            HOST,
            "--port",
            PORT,
            "--state-dir",
            os.path.join(state_root, "gateway"),
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    wait_for_health(GATEWAY_URL)
    return gateway_proc


def submit_sequence_via_tui(mode: str, state_dir: str, texts: list[str]) -> None:
    env = os.environ.copy()
    env["TERM"] = "xterm-256color"
    env["LOBSTER_WAKU_GATEWAY_URL"] = GATEWAY_URL
    env["LOBSTER_TUI_STATE_DIR"] = state_dir
    env["LOBSTER_TUI_SMOKE_SCRIPT"] = "\n".join(texts)
    isolate_tui_generated_output(env)
    subprocess.run(
        [TUI_BIN, "--mode", mode],
        cwd=ROOT,
        env=env,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=12.0,
    )


def submit_via_tui(mode: str, state_dir: str, text: str) -> None:
    submit_sequence_via_tui(mode, state_dir, [text])


def main() -> int:
    global PORT, GATEWAY_URL, WEB_GENERATED_DIR
    PORT = reserve_port()
    GATEWAY_URL = f"http://{HOST}:{PORT}"
    ensure_binaries()
    state_root = tempfile.mkdtemp(prefix="lobster-tui-smoke.")
    WEB_GENERATED_DIR = os.path.join(state_root, "web-generated")
    gateway_proc = start_gateway(state_root)
    global SESSION_TOKENS
    SESSION_TOKENS = {
        resident_id: issue_session_token(resident_id) for resident_id in ("rsaga", "tiyan")
    }
    try:
        world_state_dir = os.path.join(state_root, "tui-world")
        world_baseline = run_smoke_json("world", world_state_dir)
        assert_snapshot_contract(
            "world-baseline-snapshot",
            world_baseline,
            expected_surface="World",
            expected_active="room:world:lobby",
            required_panels=["status", "switcher", "scene", "transcript", "input"],
        )
        seed_room_message()
        seeded_world_tail = wait_for_tail_message(
            "user:rsaga", "room:world:lobby", SEEDED_TEXT
        )
        if not any(message["text"] == SEEDED_TEXT for message in seeded_world_tail["messages"]):
            fail("world-tail-after-seed", json.dumps(seeded_world_tail, ensure_ascii=False))
        user_state_dir = os.path.join(state_root, "tui-user")
        user_baseline = run_smoke_json("user", user_state_dir)
        assert_snapshot_contract(
            "user-baseline-snapshot",
            user_baseline,
            expected_surface="CityPublic",
            expected_active="room:city:core-harbor:lobby",
            required_panels=["status", "switcher", "scene", "profile", "transcript", "input"],
        )
        workbench_state_dir = os.path.join(state_root, "tui-workbench")
        workbench_baseline = run_smoke_json("workbench", workbench_state_dir)
        assert_snapshot_contract(
            "workbench-baseline-snapshot",
            workbench_baseline,
            expected_surface="CityPublic",
            expected_active="room:city:core-harbor:lobby",
            required_panels=["status", "switcher", "scene", "profile", "transcript", "input"],
        )
        stable_user_snapshot = {
            "surface_kind": user_baseline["surface_kind"],
            "active_conversation_id": user_baseline["active_conversation_id"],
            "selected_conversation_id": user_baseline["selected_conversation_id"],
            "visible_panels": user_baseline["visible_panels"],
        }
        stable_workbench_snapshot = {
            "surface_kind": workbench_baseline["surface_kind"],
            "active_conversation_id": workbench_baseline["active_conversation_id"],
            "selected_conversation_id": workbench_baseline["selected_conversation_id"],
            "visible_panels": workbench_baseline["visible_panels"],
        }
        if stable_user_snapshot != stable_workbench_snapshot:
            fail(
                "workbench-user-alias-drift",
                "\n--- user ---\n"
                + json.dumps(user_baseline, ensure_ascii=False, indent=2)
                + "\n--- workbench ---\n"
                + json.dumps(workbench_baseline, ensure_ascii=False, indent=2),
            )
        seed_city_message()
        seeded_user_tail = wait_for_tail_message(
            "user:tiyan", "room:city:core-harbor:lobby", USER_SEEDED_TEXT
        )
        if not any(message["text"] == USER_SEEDED_TEXT for message in seeded_user_tail["messages"]):
            fail("user-tail-after-seed", json.dumps(seeded_user_tail, ensure_ascii=False))
        submit_via_tui("user", user_state_dir, USER_TUI_SUBMIT_TEXT)
        user_tail = wait_for_tail_message(
            "user:tiyan", "room:city:core-harbor:lobby", USER_TUI_SUBMIT_TEXT
        )
        if not any(message["text"] == USER_TUI_SUBMIT_TEXT for message in user_tail["messages"]):
            fail("user-tail-after-tui-send", json.dumps(user_tail, ensure_ascii=False))
        submit_sequence_via_tui(
            "user", user_state_dir, ["/world", USER_WORLD_COMMAND_TEXT]
        )
        user_world_tail = wait_for_tail_message(
            "user:tiyan", "room:world:lobby", USER_WORLD_COMMAND_TEXT
        )
        if not any(
            message["text"] == USER_WORLD_COMMAND_TEXT
            for message in user_world_tail["messages"]
        ):
            fail("user-world-command-tail", json.dumps(user_world_tail, ensure_ascii=False))
        submit_sequence_via_tui(
            "user", user_state_dir, ["/open 3", USER_DIRECT_COMMAND_TEXT]
        )
        user_direct_tail = wait_for_tail_message(
            "user:tiyan", "dm:guide:tiyan", USER_DIRECT_COMMAND_TEXT
        )
        if not any(
            message["text"] == USER_DIRECT_COMMAND_TEXT
            for message in user_direct_tail["messages"]
        ):
            fail(
                "user-open3-direct-tail",
                json.dumps(user_direct_tail, ensure_ascii=False),
            )
        submit_sequence_via_tui(
            "user", user_state_dir, ["/dm builder", USER_DM_COMMAND_TEXT]
        )
        user_dm_tail = wait_for_tail_message(
            "user:tiyan", "dm:builder:tiyan", USER_DM_COMMAND_TEXT
        )
        if not any(
            message["text"] == USER_DM_COMMAND_TEXT for message in user_dm_tail["messages"]
        ):
            fail(
                "user-dm-builder-tail",
                json.dumps(user_dm_tail, ensure_ascii=False),
            )
        user_rooms = rooms_json("user:tiyan")
        if '"dm:builder:tiyan"' not in user_rooms or '"dm:tiyan:builder"' in user_rooms:
            fail("user-dm-builder-canonical", user_rooms)
        submit_via_tui("world", world_state_dir, WORLD_TUI_SUBMIT_TEXT)
        world_tail = wait_for_tail_message(
            "user:rsaga", "room:world:lobby", WORLD_TUI_SUBMIT_TEXT
        )
        if not any(message["text"] == WORLD_TUI_SUBMIT_TEXT for message in world_tail["messages"]):
            fail("world-tail-after-tui-send", json.dumps(world_tail, ensure_ascii=False))
        direct_state_dir = os.path.join(state_root, "tui-direct")
        direct_baseline = run_smoke_json("direct", direct_state_dir)
        assert_snapshot_contract(
            "direct-baseline-snapshot",
            direct_baseline,
            expected_surface="ResidenceDirect",
            expected_active=DIRECT_CONVERSATION_ID,
            required_panels=["status", "actions", "scene", "transcript", "input"],
            forbidden_panels=["profile"],
        )
        seed_direct_message()
        seeded_direct_tail = wait_for_tail_message(
            f"user:{DIRECT_RESIDENT_ID}", DIRECT_CONVERSATION_ID, DIRECT_TEXT
        )
        if not any(message["text"] == DIRECT_TEXT for message in seeded_direct_tail["messages"]):
            fail("direct-tail-after-seed", json.dumps(seeded_direct_tail, ensure_ascii=False))
        direct_rooms = rooms_json(f"user:{DIRECT_RESIDENT_ID}")
        if f'"{DIRECT_CONVERSATION_ID}"' not in direct_rooms or '"dm:tiyan:guide"' in direct_rooms:
            fail("direct-rooms-canonical", direct_rooms)

        gateway_proc.send_signal(signal.SIGTERM)
        gateway_proc.wait(timeout=5)
        gateway_proc = start_gateway(state_root)

        restarted_user_tail = tail_json("user:tiyan", "room:city:core-harbor:lobby")
        if not any(
            message["text"] == USER_TUI_SUBMIT_TEXT
            for message in restarted_user_tail["messages"]
        ):
            fail(
                "user-tail-after-restart",
                json.dumps(restarted_user_tail, ensure_ascii=False),
            )
        restarted_user_dm_tail = tail_json("user:tiyan", "dm:builder:tiyan")
        if not any(
            message["text"] == USER_DM_COMMAND_TEXT
            for message in restarted_user_dm_tail["messages"]
        ):
            fail(
                "user-dm-tail-after-restart",
                json.dumps(restarted_user_dm_tail, ensure_ascii=False),
            )
        restarted_world_tail = tail_json("user:rsaga", "room:world:lobby")
        if not any(
            message["text"] == WORLD_TUI_SUBMIT_TEXT
            for message in restarted_world_tail["messages"]
        ):
            fail(
                "world-tail-after-restart",
                json.dumps(restarted_world_tail, ensure_ascii=False),
            )
        admin_state_dir = os.path.join(state_root, "tui-admin")
        admin_baseline = run_smoke_json("admin", admin_state_dir)
        assert_snapshot_contract(
            "admin-baseline-snapshot",
            admin_baseline,
            expected_surface="CityPublic",
            expected_active="room:city:aurora-hub:announcements",
            required_panels=["status", "switcher", "scene", "profile", "transcript", "input"],
        )
        seed_governance_message()
        seeded_admin_tail = wait_for_tail_message(
            "user:rsaga",
            "room:city:aurora-hub:announcements",
            ADMIN_SEEDED_TEXT,
        )
        if not any(
            message["text"] == ADMIN_SEEDED_TEXT
            for message in seeded_admin_tail["messages"]
        ):
            fail("admin-tail-after-seed", json.dumps(seeded_admin_tail, ensure_ascii=False))
        submit_sequence_via_tui(
            "admin", admin_state_dir, ["/world", ADMIN_WORLD_COMMAND_TEXT]
        )
        admin_world_tail = wait_for_tail_message(
            "user:rsaga", "room:world:lobby", ADMIN_WORLD_COMMAND_TEXT
        )
        if not any(
            message["text"] == ADMIN_WORLD_COMMAND_TEXT
            for message in admin_world_tail["messages"]
        ):
            fail("admin-world-command-tail", json.dumps(admin_world_tail, ensure_ascii=False))
        submit_sequence_via_tui(
            "admin", admin_state_dir, ["/governance", ADMIN_GOVERNANCE_COMMAND_TEXT]
        )
        admin_governance_tail = wait_for_tail_message(
            "user:rsaga",
            "room:city:aurora-hub:announcements",
            ADMIN_GOVERNANCE_COMMAND_TEXT,
        )
        if not any(
            message["text"] == ADMIN_GOVERNANCE_COMMAND_TEXT
            for message in admin_governance_tail["messages"]
        ):
            fail(
                "admin-governance-command-tail",
                json.dumps(admin_governance_tail, ensure_ascii=False),
            )
        submit_via_tui("admin", admin_state_dir, ADMIN_TUI_SUBMIT_TEXT)
        admin_tail = wait_for_tail_message(
            "user:rsaga",
            "room:city:aurora-hub:announcements",
            ADMIN_TUI_SUBMIT_TEXT,
        )
        if not any(
            message["text"] == ADMIN_TUI_SUBMIT_TEXT for message in admin_tail["messages"]
        ):
            fail("admin-tail-after-tui-send", json.dumps(admin_tail, ensure_ascii=False))
        live_world_state_dir = os.path.join(state_root, "tui-world-live")
        live_world_session = LiveTuiSession("world", live_world_state_dir)
        try:
            time.sleep(1.5)
            live_world_session.assert_alive("world-running-session-start")
            gateway_proc.send_signal(signal.SIGTERM)
            gateway_proc.wait(timeout=5)
            time.sleep(1.0)
            live_world_session.assert_alive("world-running-session-after-gateway-stop")

            live_world_session.submit([RUNNING_WORLD_OFFLINE_SEND_TEXT])
            time.sleep(1.0)
            local_world_offline_output = wait_for_dump_contains(
                "world",
                live_world_state_dir,
                required_markers=[],
                required_texts=[RUNNING_WORLD_OFFLINE_SEND_TEXT],
                gateway_url=None,
            )
            if RUNNING_WORLD_OFFLINE_SEND_TEXT not in local_world_offline_output:
                fail(
                    "world-local-store-after-offline-send",
                    local_world_offline_output,
                )

            gateway_proc = start_gateway(state_root)
            time.sleep(2.0)
            live_world_session.assert_alive("world-running-session-after-gateway-restart")
            world_offline_recovery_tail = wait_for_tail_message(
                "user:rsaga",
                "room:world:lobby",
                RUNNING_WORLD_OFFLINE_SEND_TEXT,
            )
            if not any(
                message["text"] == RUNNING_WORLD_OFFLINE_SEND_TEXT
                for message in world_offline_recovery_tail["messages"]
            ):
                fail(
                    "world-tail-after-offline-send-recovery",
                    json.dumps(world_offline_recovery_tail, ensure_ascii=False),
                )

            send_room_message(RUNNING_WORLD_RECOVERY_TEXT)
            world_recovery_tail = wait_for_tail_message(
                "user:rsaga", "room:world:lobby", RUNNING_WORLD_RECOVERY_TEXT
            )
            if not any(
                message["text"] == RUNNING_WORLD_RECOVERY_TEXT
                for message in world_recovery_tail["messages"]
            ):
                fail(
                    "world-tail-after-running-session-recovery-send",
                    json.dumps(world_recovery_tail, ensure_ascii=False),
                )

            time.sleep(3.0)
            live_world_session.assert_alive("world-running-session-after-recovery-send")

            gateway_proc.send_signal(signal.SIGTERM)
            gateway_proc.wait(timeout=5)
            local_world_recovery_output = wait_for_dump_contains(
                "world",
                live_world_state_dir,
                required_markers=[],
                required_texts=[RUNNING_WORLD_RECOVERY_TEXT],
                gateway_url=None,
            )
            if RUNNING_WORLD_RECOVERY_TEXT not in local_world_recovery_output:
                fail(
                    "world-local-store-after-running-session-recovery",
                    local_world_recovery_output,
                )
            gateway_proc = start_gateway(state_root)
        finally:
            live_world_session.close()

        restarted_direct_seed_tail = tail_json(
            f"user:{DIRECT_RESIDENT_ID}", DIRECT_CONVERSATION_ID
        )
        if not any(message["text"] == DIRECT_TEXT for message in restarted_direct_seed_tail["messages"]):
            fail(
                "direct-tail-after-restart",
                json.dumps(restarted_direct_seed_tail, ensure_ascii=False),
            )

        submit_via_tui("direct", direct_state_dir, TUI_SUBMIT_TEXT)
        direct_tail = wait_for_tail_message(
            f"user:{DIRECT_RESIDENT_ID}", DIRECT_CONVERSATION_ID, TUI_SUBMIT_TEXT
        )
        if not any(message["text"] == TUI_SUBMIT_TEXT for message in direct_tail["messages"]):
            fail("direct-tail-after-tui-send", json.dumps(direct_tail, ensure_ascii=False))
        gateway_proc.send_signal(signal.SIGTERM)
        gateway_proc.wait(timeout=5)
        gateway_proc = start_gateway(state_root)

        restarted_admin_tail = tail_json(
            "user:rsaga", "room:city:aurora-hub:announcements"
        )
        if not any(
            message["text"] == ADMIN_TUI_SUBMIT_TEXT
            for message in restarted_admin_tail["messages"]
        ):
            fail(
                "admin-tail-after-restart",
                json.dumps(restarted_admin_tail, ensure_ascii=False),
            )
        if not any(
            message["text"] == ADMIN_GOVERNANCE_COMMAND_TEXT
            for message in restarted_admin_tail["messages"]
        ):
            fail(
                "admin-governance-command-tail-after-restart",
                json.dumps(restarted_admin_tail, ensure_ascii=False),
            )
        restarted_direct_tail = tail_json(
            f"user:{DIRECT_RESIDENT_ID}", DIRECT_CONVERSATION_ID
        )
        if not any(message["text"] == TUI_SUBMIT_TEXT for message in restarted_direct_tail["messages"]):
            fail(
                "direct-tail-after-tui-restart",
                json.dumps(restarted_direct_tail, ensure_ascii=False),
            )
        return 0
    finally:
        if gateway_proc.poll() is None:
            gateway_proc.send_signal(signal.SIGTERM)
            gateway_proc.wait(timeout=5)
        shutil.rmtree(state_root, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
