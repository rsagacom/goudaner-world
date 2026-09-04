#!/usr/bin/env node
// smoke-webpush-e2e.mjs — WebPush 端到端冒烟（蓝图序 2 验收）。
//
// 链路：真实网关二进制 + 静态页 + Playwright 真浏览器。
//   1. 居民经 dev inline OTP 注册并拿到 Bearer；
//   2. 页面内用 WebCrypto 生成订阅密钥（ECDH P-256 + 16B auth），经
//      POST /v1/push/subscribe 注册到网关（含网关侧 ECDH 试算）；
//   3. CLI/Agent 通道（POST /v1/cli/send，agent token 鉴权）发消息触发投递；
//   4. 假推送服务捕获 aes128gcm 帧，回传页面；
//   5. 页面用 WebCrypto 按 RFC 8291 解密，断言载荷包含消息文本。
//
// 这验证的是「网关加密帧 ↔ 浏览器 WebCrypto 解密」的真实互操作（不依赖
// FCM/APNs；推送服务的最终投递属生产 HTTPS 验收）。
//
// 运行：node scripts/smoke-webpush-e2e.mjs
// 前置：cargo build（网关二进制）+ Playwright 浏览器已安装。

import { spawn } from "node:child_process";
import { constants as fsConstants } from "node:fs";
import { access, mkdtemp, rm } from "node:fs/promises";
import http from "node:http";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const ROOT_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const WEB_ROOT = path.join(ROOT_DIR, "apps", "lobster-web-shell");
const HOST = "127.0.0.1";
const GATEWAY_BIN =
  process.env.GATEWAY_BIN || path.join(ROOT_DIR, "target", "debug", "lobster-waku-gateway");
const AGENT_TOKEN = "e2e-agent-token";

function b64urlFromBytes(bytes) {
  return Buffer.from(bytes).toString("base64url");
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, HOST, () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

async function waitForHttp(url, attempts = 80) {
  for (let index = 0; index < attempts; index += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // keep waiting
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`timed out waiting for ${url}`);
}

async function assertExecutable(filePath, label) {
  try {
    await access(filePath, fsConstants.X_OK);
  } catch (error) {
    throw new Error(`${label} binary not found or not executable: ${filePath}`, { cause: error });
  }
}

async function terminate(child) {
  if (!child || child.killed) return;
  child.kill("SIGTERM");
  await new Promise((resolve) => {
    const timeout = setTimeout(() => child.kill("SIGKILL"), 2000);
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve();
    });
  });
}

function createStaticServer() {
  const server = http.createServer(async (request, response) => {
    try {
      const url = new URL(request.url, "http://localhost");
      let filePath = path.join(WEB_ROOT, url.pathname === "/" ? "index.html" : url.pathname);
      if (!filePath.startsWith(WEB_ROOT)) {
        response.writeHead(403);
        response.end();
        return;
      }
      const content = await import("node:fs/promises").then((fs) => fs.readFile(filePath));
      response.writeHead(200);
      response.end(content);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return server;
}

async function main() {
  await assertExecutable(GATEWAY_BIN, "gateway");

  const tempDir = await mkdtemp(path.join(os.tmpdir(), "lobster-webpush-e2e-"));
  const gatewayPort = await freePort();
  const stateDir = path.join(tempDir, "state");

  // 假推送服务：捕获网关投递的加密帧
  let captured = null;
  let capturedSignal;
  const capturedPromise = new Promise((resolve) => {
    capturedSignal = resolve;
  });
  const pushService = http.createServer((request, response) => {
    const chunks = [];
    request.on("data", (chunk) => chunks.push(chunk));
    request.on("end", () => {
      const headers = {};
      for (const [name, value] of Object.entries(request.headers)) headers[name] = String(value);
      captured = {
        url: request.url,
        contentEncoding: headers["content-encoding"] || "",
        authorization: headers.authorization || "",
        ttl: headers.ttl || "",
        body: Buffer.concat(chunks),
      };
      response.writeHead(201);
      response.end();
      capturedSignal();
    });
  });
  const pushPort = await freePort();
  await new Promise((resolve) => pushService.listen(pushPort, HOST, resolve));

  const gatewayEnv = {
    ...process.env,
    LOBSTER_DEV_EMAIL_OTP_INLINE: "1",
    LOBSTER_DEV_AUTH_BYPASS: "1",
    LOBSTER_AGENT_TOKENS: `agent:bench=${AGENT_TOKEN}`,
  };
  const gateway = spawn(
    GATEWAY_BIN,
    ["--host", HOST, "--port", String(gatewayPort), "--state-dir", stateDir],
    {
      cwd: ROOT_DIR,
      env: gatewayEnv,
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  const gatewayBase = `http://${HOST}:${gatewayPort}`;
  await waitForHttp(`${gatewayBase}/health`);

  const staticServer = createStaticServer();
  const staticPort = await freePort();
  await new Promise((resolve) => staticServer.listen(staticPort, HOST, resolve));
  const pageBase = `http://${HOST}:${staticPort}`;

  const browser = await chromium.launch({ headless: true });
  try {
    // 1. 居民注册（dev inline OTP 直接返回验证码）
    const email = `e2e-${Date.now()}@push.example`;
    const requestResponse = await fetch(`${gatewayBase}/v1/auth/email-otp/request`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, nickname: "E2E" }),
    });
    const requestPayload = await requestResponse.json();
    if (!requestPayload.dev_code) throw new Error("dev inline OTP did not return dev_code");
    const verifyResponse = await fetch(`${gatewayBase}/v1/auth/email-otp/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        challenge_id: requestPayload.challenge_id,
        code: requestPayload.dev_code,
      }),
    });
    const session = await verifyResponse.json();
    if (!session.session_token) throw new Error(`verify failed: ${JSON.stringify(session)}`);
    const token = session.session_token;
    const residentId = session.resident_id;

    // 2. 打开住宅页，写入会话
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error)));
    await page.addInitScript(
      ([tokenValue, identity]) => {
        window.localStorage.setItem("lobster-session-token", tokenValue);
        window.localStorage.setItem("lobster-identity", identity);
      },
      [token, "E2E"],
    );
    await page.goto(`${pageBase}/index.html?gateway=${encodeURIComponent(gatewayBase)}`, {
      waitUntil: "networkidle",
    });
    await page.waitForTimeout(400);

    // 3. 页面内生成 WebPush 订阅密钥（WebCrypto——与真实浏览器同一原语）
    const keys = await page.evaluate(async () => {
      const toB64url = (buffer) => {
        let raw = "";
        for (const byte of new Uint8Array(buffer)) raw += String.fromCharCode(byte);
        return btoa(raw).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
      };
      const pair = await crypto.subtle.generateKey(
        { name: "ECDH", namedCurve: "P-256" },
        true,
        ["deriveBits"],
      );
      const rawPublic = await crypto.subtle.exportKey("raw", pair.publicKey);
      const auth = crypto.getRandomValues(new Uint8Array(16));
      window.__webpushE2e = { privateKey: pair.privateKey, publicKeyRaw: new Uint8Array(rawPublic) };
      return { p256dh: toB64url(rawPublic), auth: toB64url(auth) };
    });

    // 4. 订阅（网关侧会做 ECDH 试算校验）
    const endpoint = `http://${HOST}:${pushPort}/push/send/e2e`;
    const subscribeResponse = await fetch(`${gatewayBase}/v1/push/subscribe`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ endpoint, keys }),
    });
    if (subscribeResponse.status !== 201) {
      const detail = await subscribeResponse.text();
      throw new Error(`subscribe failed: ${subscribeResponse.status} ${detail}`);
    }

    // 5. CLI/Agent 通道发消息 → 触发推送投递
    const messageText = `e2e push payload ${Date.now()}`;
    const cliResponse = await fetch(`${gatewayBase}/v1/cli/send`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${AGENT_TOKEN}`,
      },
      body: JSON.stringify({ from: "agent:bench", to: `user:${residentId}`, text: messageText }),
    });
    if (!cliResponse.ok) throw new Error(`cli send failed: ${cliResponse.status}`);

    await capturedPromise;
    if (!captured.contentEncoding.includes("aes128gcm")) {
      throw new Error("push frame missing Content-Encoding: aes128gcm");
    }
    if (!captured.authorization.startsWith("vapid t=")) {
      throw new Error("push frame missing VAPID authorization");
    }
    if (captured.ttl !== "86400") throw new Error("push frame missing TTL");
    const frame = new Uint8Array(captured.body);
    if (frame.length <= 86 + 16) throw new Error("push frame too small");
    if (frame[16] !== 0x00 || frame[17] !== 0x00 || frame[18] !== 0x10 || frame[19] !== 0x00) {
      throw new Error("push frame record size is not 4096");
    }
    if (frame[20] !== 65 || frame[21] !== 0x04) throw new Error("push frame keyid malformed");

    // 6. 页面内按 RFC 8291 用 WebCrypto 解密帧（真实浏览器互操作证据）
    const decrypted = await page.evaluate(async ({ frameB64, keysB64 }) => {
      const fromB64url = (value) => {
        const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
        const raw = atob(normalized + "=".repeat((4 - (normalized.length % 4)) % 4));
        return Uint8Array.from(raw, (ch) => ch.charCodeAt(0));
      };
      const concat = (...arrays) => {
        const total = arrays.reduce((sum, item) => sum + item.length, 0);
        const out = new Uint8Array(total);
        let offset = 0;
        for (const item of arrays) {
          out.set(item, offset);
          offset += item.length;
        }
        return out;
      };
      const frame = fromB64url(frameB64);
      const salt = frame.slice(0, 16);
      const keyIdLength = frame[20];
      const asPublic = frame.slice(21, 21 + keyIdLength);
      const ciphertext = frame.slice(21 + keyIdLength);

      const { privateKey, publicKeyRaw } = window.__webpushE2e;
      const ecdhSecret = await crypto.subtle.deriveBits(
        { name: "ECDH", public: await crypto.subtle.importKey("raw", asPublic, { name: "ECDH", namedCurve: "P-256" }, false, []) },
        privateKey,
        256,
      );
      const ikm = await crypto.subtle.deriveBits(
        {
          name: "HKDF",
          hash: "SHA-256",
          salt: fromB64url(keysB64.auth),
          info: concat(new TextEncoder().encode("WebPush: info"), new Uint8Array([0]), publicKeyRaw, asPublic),
        },
        await crypto.subtle.importKey("raw", ecdhSecret, "HKDF", false, ["deriveBits"]),
        256,
      );
      const cek = await crypto.subtle.deriveBits(
        {
          name: "HKDF",
          hash: "SHA-256",
          salt,
          info: concat(new TextEncoder().encode("Content-Encoding: aes128gcm"), new Uint8Array([0])),
        },
        await crypto.subtle.importKey("raw", ikm, "HKDF", false, ["deriveBits"]),
        128,
      );
      const nonce = await crypto.subtle.deriveBits(
        {
          name: "HKDF",
          hash: "SHA-256",
          salt,
          info: concat(new TextEncoder().encode("Content-Encoding: nonce"), new Uint8Array([0])),
        },
        await crypto.subtle.importKey("raw", ikm, "HKDF", false, ["deriveBits"]),
        96,
      );
      const record = await crypto.subtle.decrypt(
        { name: "AES-GCM", iv: nonce },
        await crypto.subtle.importKey("raw", cek, "AES-GCM", false, ["decrypt"]),
        ciphertext,
      );
      const plaintextBytes = new Uint8Array(record);
      return new TextDecoder().decode(plaintextBytes.slice(0, plaintextBytes.length - 1));
    }, {
      frameB64: b64urlFromBytes(frame),
      keysB64: { auth: keys.auth, p256dh: keys.p256dh },
    });

    const payload = JSON.parse(decrypted);
    if (!payload.body.includes(messageText)) {
      throw new Error(`decrypted payload missing message text: ${decrypted}`);
    }

    if (pageErrors.length) {
      throw new Error(`uncaught page errors: ${pageErrors.join(" | ")}`);
    }

    console.log("=== webpush e2e smoke passed ===");
    console.log(`endpoint: ${endpoint}`);
    console.log(`decrypted payload: ${decrypted.slice(0, 120)}`);
    console.log(`frame: ${frame.length} bytes, aes128gcm, TTL 86400, VAPID ES256`);
  } finally {
    await browser.close();
    pushService.close();
    staticServer.close();
    await terminate(gateway);
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
