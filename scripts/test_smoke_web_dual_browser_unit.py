#!/usr/bin/env python3
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "smoke-web-dual-browser.mjs"


def main() -> int:
    assert SCRIPT.exists(), f"missing web dual browser smoke script: {SCRIPT}"
    text = SCRIPT.read_text(encoding="utf-8")

    assert text.startswith("#!/usr/bin/env node")
    assert 'import { spawn } from "node:child_process";' in text
    assert 'import { constants as fsConstants } from "node:fs";' in text
    assert 'access, mkdtemp, rm } from "node:fs/promises";' in text
    assert 'import net from "node:net";' in text
    assert 'from "playwright"' not in text
    assert 'const { chromium } = await import("playwright");' in text
    assert 'const WEB_ROOT = path.join(ROOT_DIR, "apps", "lobster-web-shell");' in text
    assert 'const KEEP_STATE = process.env.KEEP_STATE === "1";' in text
    assert 'const SKIP_BUILD = process.env.SKIP_BUILD === "1";' in text
    assert 'process.env.GATEWAY_BIN || path.join(ROOT_DIR, "target", "debug", "lobster-waku-gateway")' in text
    assert "function spawnChecked(command, args, options = {})" in text
    assert 'cwd: ROOT_DIR' in text
    assert "function freePort()" in text
    assert "async function waitForHttp(url, attempts = 80)" in text
    assert "async function assertExecutable(filePath, label)" in text
    assert "fsConstants.X_OK" in text
    assert 'throw new Error(`${label} binary not found or not executable: ${filePath}`, { cause: error });' in text
    assert "async function terminate(child)" in text
    assert 'child.kill("SIGTERM")' in text
    assert 'child.kill("SIGKILL")' in text
    assert "async function selectPublicRoom(page)" in text
    assert "async function submitComposer(page, text)" in text
    assert "async function failNextMessagePost(page)" in text
    assert '"**/v1/shell/message"' in text
    assert '"smoke forced message failure"' in text
    assert "async function expectMessageSide(page, text, side)" in text
    assert "async function expectFailedPendingMessage(page, text)" in text
    assert "async function clickPendingRetry(page, text)" in text
    assert "async function expectEditedMessage(page, editedText, side)" in text
    assert "async function clickMessageAction(page, text, side, action)" in text
    assert "async function expectRecalledMessage(page, { previousText, side })" in text
    assert "async function expectAdminSessionExpiry(page, forcedFailures)" in text
    assert 'localStorage.setItem("lobster-session-token", "expired-session-fixture")' in text
    assert 'localStorage.setItem("lobster-identity", "admin-browser")' in text
    assert 'await adminPage.route("**/v1/admin/summary"' in text
    assert 'status: 401' in text
    assert 'invalid or expired session' in text
    assert 'await adminPage.goto(`${webUrl}/admin-ds.html?gateway=${encodeURIComponent(gatewayUrl)}`);' in text
    assert 'await expectAdminSessionExpiry(adminPage, adminAuthFailure);' in text
    assert 'admin-ds auth expiry: 401 -> visitor/login HUD -> overlay' in text
    assert '"消息已撤回"' in text
    assert 'await run("cargo", ["build", "--manifest-path", path.join(ROOT_DIR, "Cargo.toml"), "-p", "lobster-waku-gateway"])' in text
    assert text.index("await assertExecutable(GATEWAY_BIN, \"gateway\");") < text.index('const stateRoot = await mkdtemp(path.join(os.tmpdir(), "lobster-web-dual-browser."));')
    assert 'const stateRoot = await mkdtemp(path.join(os.tmpdir(), "lobster-web-dual-browser."));' in text
    assert 'gateway = spawnChecked(GATEWAY_BIN, [' in text
    assert 'env: { LOBSTER_DEV_AUTH_BYPASS: "1" }' in text
    assert 'web = spawnChecked("python3", [' in text
    assert 'browser = await chromium.launch({ headless: true });' in text
    assert 'adminContext = await browser.newContext();' in text
    assert 'await adminContext?.close().catch(() => {});' in text
    assert 'label === "admin-ds" && message.type() === "error" && message.text().includes("401")' in text
    assert 'const browserDiagnostics = [];' in text
    assert 'let expectedBrowser503s = 0;' in text
    assert 'expectedBrowser503s > 0' in text
    assert 'expectedBrowser503s -= 1;' in text
    assert 'expectedBrowser503s = 1;' in text
    assert 'browserDiagnostics.push' in text
    assert 'if (browserDiagnostics.length > 0)' in text
    assert 'throw new Error(`browser diagnostics: ${browserDiagnostics.join(" | ")}`);' in text
    assert 'await indexPage.goto(`${webUrl}/index.html?gateway=${encodeURIComponent(gatewayUrl)}&identity=qa-a&qa=browser`);' in text
    assert 'await creativePage.goto(`${webUrl}/creative.html?gateway=${encodeURIComponent(gatewayUrl)}&identity=qa-b&qa=browser`);' in text
    assert 'await clickMessageAction(indexPage, textA, "self", "edit");' in text
    assert 'await clickMessageAction(indexPage, editedTextA, "self", "recall");' in text
    assert "await failNextMessagePost(indexPage);" in text
    assert "await clickPendingRetry(indexPage, retryText);" in text
    assert 'console.log("== web dual browser smoke passed ==");' in text
    assert "await browser?.close().catch(() => {});" in text
    assert "await terminate(web);" in text
    assert "await terminate(gateway);" in text
    assert "await rm(stateRoot, { recursive: true, force: true });" in text
    assert "console.error(error?.message || error);" in text
    return 0


if __name__ == "__main__":
    sys.exit(main())
