/* ============================================================
   push-client.test.mjs — WebPush 订阅客户端合同测试
   覆盖：pushCapabilityState 决策矩阵、urlBase64ToUint8Array、
        buildSubscribePayload、createPushClient DOM 行为、
        index/creative 页推送钮钉。
   ============================================================ */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const clientUrl = new URL("../shell-push-client.js", import.meta.url);
const {
  pushCapabilityState,
  urlBase64ToUint8Array,
  buildSubscribePayload,
  createPushClient,
} = await import(pathToFileURL(clientUrl.pathname).href);

test("pushCapabilityState 决策矩阵", () => {
  assert.equal(pushCapabilityState({}), "unsupported");
  assert.equal(pushCapabilityState({ secureContext: true }), "unsupported");
  assert.equal(
    pushCapabilityState({ secureContext: true, pushSupported: true }),
    "unsupported",
  );
  assert.equal(
    pushCapabilityState({
      secureContext: true,
      pushSupported: true,
      serviceWorkerSupported: true,
    }),
    "unsubscribed",
  );
  assert.equal(
    pushCapabilityState({
      secureContext: true,
      pushSupported: true,
      serviceWorkerSupported: true,
      permission: "denied",
    }),
    "denied",
  );
  assert.equal(
    pushCapabilityState({
      secureContext: true,
      pushSupported: true,
      serviceWorkerSupported: true,
      subscribed: true,
    }),
    "subscribed",
  );
  assert.equal(
    pushCapabilityState({
      secureContext: true,
      pushSupported: true,
      serviceWorkerSupported: true,
      permission: "denied",
      subscribed: true,
    }),
    "subscribed",
    "已订阅态优先于权限展示",
  );
});

test("urlBase64ToUint8Array 解析 URL-safe 无填充编码", () => {
  // RFC 8291 §5 salt
  const salt = urlBase64ToUint8Array("DGv6ra1nlYgDCS1FRnbzlw");
  assert.equal(salt.length, 16);
  assert.equal(salt[0], 0x0c);
  // - 和 _ 的映射
  const roundtrip = Buffer.from(urlBase64ToUint8Array("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"));
  assert.equal(roundtrip.length, 65);
  assert.equal(roundtrip[0], 0x04);
});

test("buildSubscribePayload 校验必需字段", () => {
  assert.throws(() => buildSubscribePayload(null));
  assert.throws(() => buildSubscribePayload({ endpoint: "https://push.example" }));
  const payload = buildSubscribePayload({
    endpoint: "https://push.example/abc",
    keys: { p256dh: "K", auth: "A" },
  });
  assert.deepEqual(payload, {
    endpoint: "https://push.example/abc",
    keys: { p256dh: "K", auth: "A" },
  });
});

// ---- createPushClient DOM 行为（fake document） ----

function fakeDocHarness({ storedPermission = "default" } = {}) {
  const createElement = (tag) => ({
    tagName: tag.toUpperCase(),
    className: "",
    hidden: false,
    textContent: "",
    title: "",
    dataset: {},
    attrs: {},
    children: [],
    listeners: new Map(),
    appendChild(child) {
      this.children.push(child);
      return child;
    },
    replaceWith() {},
    setAttribute(name, value) {
      if (name === "aria-pressed") this.attrs[name] = String(value);
      else this.attrs[name] = String(value);
    },
    getAttribute(name) {
      return this.attrs[name] ?? null;
    },
    addEventListener(type, handler) {
      this.listeners.set(type, handler);
    },
    click() {
      this.listeners.get("click")?.({});
    },
    classList: {
      toggle() {},
    },
  });
  const doc = {
    createElement,
    querySelector: (selector) =>
      selector === "[data-push-toggle]" ? createElement("button") : null,
  };
  const registration = {
    pushManager: {
      getSubscription: async () => null,
      subscribe: async () => ({
        endpoint: "https://push.example/abc",
        keys: { p256dh: "K", auth: "A" },
      }),
    },
  };
  const navigatorRef = {
    serviceWorker: {
      register: async () => registration,
      ready: Promise.resolve(registration),
      getRegistration: async () => registration,
    },
  };
  const windowRef = {
    isSecureContext: true,
    PushManager: function PushManager() {},
  };
  const windowRefWithNotification = {
    ...windowRef,
    Notification: { permission: storedPermission, requestPermission: async () => storedPermission },
  };
  return { doc, navigatorRef, windowRef: windowRefWithNotification };
}

test("不支持的浏览器环境隐藏推送钮", async () => {
  const { doc, navigatorRef, windowRef } = fakeDocHarness();
  windowRef.PushManager = undefined;
  windowRef.Notification = { permission: "denied" };
  const client = createPushClient({
    document: doc,
    navigatorRef,
    windowRef,
    getGatewayUrl: () => "https://chat.example.com",
    getSessionToken: () => null,
  });
  const state = await client.init();
  assert.equal(state, "unsupported");
  assert.equal(client.element.hidden, true);
});

test("支持环境显示开关，toggle 流程走订阅与上报", async () => {
  const { doc, navigatorRef, windowRef } = fakeDocHarness({ storedPermission: "granted" });
  const posted = [];
  const fakeFetch = async (url, init = {}) => {
    if (url.endsWith("/v1/push/vapid-public-key")) {
      return {
        ok: true,
        json: async () => ({
          ok: true,
          // RFC 8291 §5 的应用服务器公钥（base64url）
          public_key:
            "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8",
        }),
      };
    }
    posted.push({ url, body: JSON.parse(init.body ?? "{}") });
    return { ok: true, json: async () => ({ ok: true }) };
  };

  // 注入最小 PushManager：subscribe 返回固定订阅
  let subscriptionHandler = async () => ({
    endpoint: "https://push.example/abc",
    keys: { p256dh: "K", auth: "A" },
  });
  navigatorRef.pushManager = {
    subscribe: subscriptionHandler,
  };
  navigatorRef.serviceWorker.getRegistration = async () => ({
    pushManager: {
      getSubscription: async () => null,
      subscribe: subscriptionHandler,
    },
  });
  // atob 注入（node 环境无 window.atob）
  const originalAtob = globalThis.atob;
  globalThis.atob = (value) => Buffer.from(value, "base64").toString("binary");

  try {
    const client = createPushClient({
      document: doc,
      navigatorRef,
      windowRef: { ...windowRef, fetch: fakeFetch },
      getGatewayUrl: () => "https://chat.example.com",
      getSessionToken: () => "token-1",
    });
    // fetch 注入到全局（客户端内部直接用 fetch）
    const originalFetch = globalThis.fetch;
    globalThis.fetch = fakeFetch;
    try {
      const state = await client.init();
      assert.equal(state, "unsubscribed");
      assert.equal(client.element.hidden, false);
      assert.equal(client.element.textContent, "铃");

      await client.toggle();
      assert.equal(posted.length, 1, 'toggle error title: ' + client.element.title);
      assert.equal(posted[0].url, "https://chat.example.com/v1/push/subscribe");
      assert.equal(posted[0].body.endpoint, "https://push.example/abc");
    } finally {
      globalThis.fetch = originalFetch;
    }
  } finally {
    if (originalAtob) globalThis.atob = originalAtob;
  }
});

// ---- 页面推送钮钉 ----

test("index/creative 页面含推送开关占位钮", () => {
  for (const page of ["index.html", "creative.html"]) {
    const html = readFileSync(path.join(ROOT, page), "utf8");
    assert.match(html, /data-push-toggle/, page);
    assert.match(html, /composer-push-toggle/, page);
  }
});

// ---- sw.js 静态合同 ----

test("sw.js 监听 push 与 notificationclick 并展示通知", () => {
  const sw = readFileSync(path.join(ROOT, "sw.js"), "utf8");
  assert.match(sw, /addEventListener\("push"/);
  assert.match(sw, /showNotification/);
  assert.match(sw, /addEventListener\("notificationclick"/);
  assert.match(sw, /clients\.claim/);
});

// ---- 登出静默退订（隐私加固） ----

test("disableSilently 上报退订并清除浏览器订阅，失败不阻塞登出", async () => {
  const unsubscribeSpies = { server: 0, browser: 0 };
  const createElement = (tag) => ({
    tagName: tag.toUpperCase(),
    className: "",
    hidden: false,
    textContent: "",
    title: "",
    attrs: {},
    dataset: {},
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
  const subscription = {
    endpoint: "https://push.example/abc",
    unsubscribe: async () => {
      unsubscribeSpies.browser += 1;
    },
  };
  const registration = {
    pushManager: { getSubscription: async () => subscription },
  };
  const doc = {
    createElement: (tag) => createElement(tag),
    querySelector: () => null,
  };
  const store = new Map();
  const posted = [];
  const client = createPushClient({
    document: doc,
    navigatorRef: {
      serviceWorker: { getRegistration: async () => registration },
    },
    windowRef: { isSecureContext: true, PushManager: function P() {}, Notification: { permission: "granted" } },
    getGatewayUrl: () => "https://chat.example.com",
    getSessionToken: () => "token-9",
    storage: {
      getItem: (key) => (store.has(key) ? store.get(key) : null),
      setItem: (key, value) => store.set(key, String(value)),
    },
  });
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url, init = {}) => {
    posted.push({ url, body: init.body ? JSON.parse(init.body) : null, auth: init.headers?.Authorization ?? null });
    return { ok: true, json: async () => ({ ok: true }) };
  };
  try {
    await client.disableSilently();
  } finally {
    globalThis.fetch = originalFetch;
  }
  assert.equal(unsubscribeSpies.browser, 1);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].url, "https://chat.example.com/v1/push/unsubscribe");
  assert.equal(posted[0].auth, "Bearer token-9");
  assert.equal(posted[0].body.endpoint, "https://push.example/abc");
});
