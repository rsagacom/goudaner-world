/* ============================================================
   pwa-install.test.mjs — PWA manifest 最小版 + 加桌引导合同测试
   覆盖：manifest.webmanifest 合法性与图标实存实尺寸、
        installHintState 决策矩阵、createInstallHint DOM 行为、
        index/creative 页 PWA 头部钉。
   ============================================================ */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// ---- manifest 合同 ----

const manifest = JSON.parse(readFileSync(path.join(ROOT, "manifest.webmanifest"), "utf8"));

test("manifest 基础合同：名称、独立窗口、暗色底", () => {
  assert.equal(manifest.name, "我和狗蛋儿的家（Goudaner World）");
  assert.equal(manifest.short_name, "狗蛋儿");
  assert.equal(manifest.display, "standalone");
  assert.equal(manifest.start_url, "./index.html");
  assert.equal(manifest.scope, "./");
  assert.equal(manifest.background_color, "#1a120e");
  assert.equal(manifest.theme_color, "#1a120e");
});

test("manifest 图标实存且 PNG 实际尺寸与声明一致", () => {
  assert.ok(Array.isArray(manifest.icons) && manifest.icons.length >= 3);
  const purposes = new Set(manifest.icons.map((icon) => icon.purpose));
  assert.ok(purposes.has("any"), "需要 any 图标");
  assert.ok(purposes.has("maskable"), "需要 maskable 图标");
  for (const icon of manifest.icons) {
    assert.ok(icon.src.startsWith("./assets/icons/"), icon.src);
    const file = path.join(ROOT, icon.src);
    const buffer = readFileSync(file);
    // PNG 签名 + IHDR 宽高
    assert.deepEqual([...buffer.subarray(0, 8)], [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const width = buffer.readUInt32BE(16);
    const height = buffer.readUInt32BE(20);
    assert.equal(`${width}x${height}`, icon.sizes, icon.src);
    assert.equal(icon.type, "image/png");
  }
});

// ---- 加桌引导决策矩阵 ----

const hintUrl = new URL("../shell-install-hint.js", import.meta.url);
const { installHintState, detectIosLike, createInstallHint, INSTALL_HINT_DISMISS_KEY } =
  await import(pathToFileURL(hintUrl.pathname).href);

test("installHintState 决策矩阵", () => {
  assert.equal(installHintState({}), "hidden");
  assert.equal(installHintState({ displayStandalone: true, hasInstallPrompt: true }), "hidden");
  assert.equal(installHintState({ iosStandalone: true, iosLike: true }), "hidden");
  assert.equal(installHintState({ dismissed: true, hasInstallPrompt: true }), "hidden");
  assert.equal(installHintState({ hasInstallPrompt: true }), "prompt");
  assert.equal(installHintState({ iosLike: true }), "ios");
  assert.equal(installHintState({ dismissed: true, iosLike: true }), "hidden");
});

test("detectIosLike 覆盖 iPadOS 伪装 Mac", () => {
  assert.equal(detectIosLike({ userAgent: "iPhone" }), true);
  assert.equal(detectIosLike({ userAgent: "iPad" }), true);
  assert.equal(
    detectIosLike({ userAgent: "Macintosh", maxTouchPoints: 5 }),
    true,
    "iPadOS 13+ 桌面 UA 需按触屏识别",
  );
  assert.equal(detectIosLike({ userAgent: "Macintosh", maxTouchPoints: 0 }), false);
  assert.equal(detectIosLike({ userAgent: "Android" }), false);
});

// ---- createInstallHint DOM 行为（fake document） ----

function fakeDocumentHarness({ dismissed = false } = {}) {
  const store = new Map(dismissed ? [[INSTALL_HINT_DISMISS_KEY, "1"]] : []);
  const storage = {
    getItem: (key) => (store.has(key) ? store.get(key) : null),
    setItem: (key, value) => store.set(key, String(value)),
  };
  const createElement = (tag) => ({
    tagName: tag.toUpperCase(),
    className: "",
    hidden: false,
    textContent: "",
    attrs: {},
    children: [],
    listeners: new Map(),
    appendChild(child) {
      this.children.push(child);
      return child;
    },
    setAttribute(name, value) {
      this.attrs[name] = String(value);
    },
    getAttribute(name) {
      return this.attrs[name] ?? null;
    },
    addEventListener(type, handler) {
      this.listeners.set(type, handler);
    },
    click() {
      this.listeners.get("click")?.({ target: this });
    },
  });
  const body = { ...createElement("body"), children: [] };
  const doc = { createElement, body, querySelector: () => ({}) };
  return { doc, storage, store };
}

test("prompt 状态：显示安装按钮，点击调用 deferred prompt 并收起", () => {
  const { doc, storage } = fakeDocumentHarness();
  const hint = createInstallHint({ document: doc, storage });
  const state = hint.init({ displayStandalone: false, iosLike: false, hasInstallPrompt: false });
  assert.equal(state, "hidden");
  assert.equal(hint.isOpen(), false);

  let prompted = false;
  hint.setDeferredPrompt({ prompt: () => (prompted = true) });
  assert.equal(hint.isOpen(), true);
  assert.equal(hint.action.hidden, false);
  assert.equal(hint.action.textContent, "安装");

  hint.action.click();
  assert.equal(prompted, true);
  assert.equal(hint.isOpen(), false);
});

test("iOS 状态：指引文案且无安装按钮；dismiss 持久化后不再出现", () => {
  const { doc, storage } = fakeDocumentHarness();
  const hint = createInstallHint({ document: doc, storage });
  const state = hint.init({ displayStandalone: false, iosLike: true });
  assert.equal(state, "ios");
  assert.equal(hint.isOpen(), true);
  assert.equal(hint.action.hidden, true);
  assert.ok(hint.copy.textContent.includes("添加到主屏幕"));

  hint.dismiss.click();
  assert.equal(hint.isOpen(), false);
  assert.equal(storage.getItem(INSTALL_HINT_DISMISS_KEY), "1");
  assert.equal(hint.init({ displayStandalone: false, iosLike: true }), "hidden");
});

test("已关闭用户连 beforeinstallprompt 都不再弹出", () => {
  const { doc, storage } = fakeDocumentHarness({ dismissed: true });
  const hint = createInstallHint({ document: doc, storage });
  hint.init({ displayStandalone: false, iosLike: false, hasInstallPrompt: false });
  hint.setDeferredPrompt({ prompt: () => {} });
  assert.equal(hint.isOpen(), false);
});

// ---- 页面 PWA 头部钉 ----

test("index/creative 页面注入 manifest、theme-color 与 apple-touch-icon", () => {
  for (const page of ["index.html", "creative.html"]) {
    const html = readFileSync(path.join(ROOT, page), "utf8");
    assert.match(html, /<link rel="manifest" href="\.\/manifest\.webmanifest" \/>/, page);
    assert.match(html, /<meta name="theme-color" content="#1a120e" \/>/, page);
    assert.match(html, /<link rel="apple-touch-icon" href="\.\/assets\/icons\/icon-192\.png" \/>/, page);
    assert.match(html, /<meta name="apple-mobile-web-app-title" content="狗蛋儿" \/>/, page);
  }
});
