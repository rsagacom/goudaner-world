// shell-push-client.js — WebPush 订阅客户端（蓝图序 2：推送通知）。
// 纯 DOM 模块 + 可单测决策函数，不依赖 app.js 全局。
// 视觉规范:dark-on-dark;按钮形态对齐 composer-attachment-trigger。
//
// 决策合同(pushCapabilityState):
//   - 非 secure context / 无 PushManager / 无 serviceWorker → "unsupported"(隐藏)
//   - 浏览器通知权限为 denied → "denied"(隐藏,不骚扰)
//   - 已有订阅 → "subscribed"
//   - 其余 → "unsubscribed"(显示开关,等待用户点击)
//
// 订阅流程:注册 sw.js → 取 VAPID 公钥 → pushManager.subscribe →
// POST /v1/push/subscribe(Bearer)。退订:pushManager.unsubscribe → POST。
// 任何一步失败都回落到未订阅态并展示错误文案,绝不假成功。

export function pushCapabilityState({
  secureContext = false,
  pushSupported = false,
  serviceWorkerSupported = false,
  permission = "default",
  subscribed = false,
} = {}) {
  if (!secureContext || !pushSupported || !serviceWorkerSupported) return "unsupported";
  if (subscribed) return "subscribed";
  if (permission === "denied") return "denied";
  return "unsubscribed";
}

export function urlBase64ToUint8Array(base64url) {
  const normalized = String(base64url).replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
  const raw = atob(padded);
  const output = new Uint8Array(raw.length);
  for (let index = 0; index < raw.length; index++) {
    output[index] = raw.charCodeAt(index);
  }
  return output;
}

export function buildSubscribePayload(subscription) {
  const endpoint = subscription?.endpoint;
  const keys = subscription?.keys;
  if (!endpoint || !keys?.p256dh || !keys?.auth) {
    throw new Error("订阅缺少 endpoint 或密钥");
  }
  return { endpoint, keys: { p256dh: keys.p256dh, auth: keys.auth } };
}

export function createPushClient({
  document: doc = document,
  navigatorRef = navigator,
  windowRef = window,
  getGatewayUrl = () => null,
  getSessionToken,
  onStateChange = () => {},
} = {}) {
  const button = doc.createElement("button");
  button.type = "button";
  button.className = "composer-push-toggle";
  button.dataset.pushToggle = "";
  button.setAttribute("aria-pressed", "false");
  button.textContent = "铃";
  button.hidden = true;

  function paint(state) {
    button.hidden = state === "unsupported" || state === "denied";
    button.setAttribute("aria-pressed", state === "subscribed" ? "true" : "false");
    button.classList.toggle("is-on", state === "subscribed");
    button.title =
      state === "subscribed" ? "推送已开启，点击关闭" : "开启消息推送通知";
    onStateChange(state);
  }

  async function gatewayCall(path, payload) {
    const gatewayUrl = getGatewayUrl();
    if (!gatewayUrl) throw new Error("网关未连接");
    const headers = { "Content-Type": "application/json" };
    const token = typeof getSessionToken === "function" ? getSessionToken() : null;
    if (token) headers.Authorization = `Bearer ${token}`;
    const response = await fetch(`${gatewayUrl}${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`推送服务返回 ${response.status}`);
    }
    return response.json().catch(() => ({}));
  }

  async function currentSubscription(registration) {
    const existing = await registration.pushManager.getSubscription();
    return existing || null;
  }

  async function enable() {
    const notificationRef = windowRef.Notification;
    if (!notificationRef || typeof notificationRef.requestPermission !== "function") {
      throw new Error("此环境不支持通知");
    }
    const permission = await notificationRef.requestPermission();
    if (permission !== "granted") {
      throw new Error(permission === "denied" ? "通知权限已被拒绝" : "通知权限未授予");
    }
    const registration = await navigatorRef.serviceWorker.register("./sw.js");
    await navigatorRef.serviceWorker.ready;
    const gatewayUrl = getGatewayUrl();
    if (!gatewayUrl) throw new Error("网关未连接");
    const keyResponse = await fetch(`${gatewayUrl}/v1/push/vapid-public-key`);
    if (!keyResponse.ok) throw new Error("推送公钥获取失败");
    const keyPayload = await keyResponse.json();
    if (!keyPayload.public_key) throw new Error("推送公钥缺失");
    const applicationServerKey = urlBase64ToUint8Array(keyPayload.public_key);
    const subscription = await registration.pushManager.subscribe({
      applicationServerKey,
      userVisibleOnly: true,
    });
    await gatewayCall("/v1/push/subscribe", buildSubscribePayload(subscription));
    return subscription;
  }

  async function disable() {
    const registration = await navigatorRef.serviceWorker.getRegistration("./");
    const subscription = registration ? await currentSubscription(registration) : null;
    if (subscription) {
      await gatewayCall("/v1/push/unsubscribe", { endpoint: subscription.endpoint });
      await subscription.unsubscribe();
    }
  }

  async function refresh() {
    const state = await pushClient.state();
    paint(state);
    return state;
  }

  const pushClient = {
    element: button,
    refresh,
    async state() {
      if (!getGatewayUrl()) return "unsupported";
      const secureContext = windowRef.isSecureContext !== false;
      const pushSupported = typeof windowRef.PushManager !== "undefined";
      const serviceWorkerSupported = Boolean(navigatorRef.serviceWorker);
      const notificationRef = windowRef.Notification;
      const permission =
        notificationRef && typeof notificationRef.permission === "string"
          ? notificationRef.permission
          : "denied";
      let subscribed = false;
      if (pushSupported && serviceWorkerSupported) {
        try {
          const registration = await navigatorRef.serviceWorker.getRegistration("./");
          subscribed = Boolean(registration && (await currentSubscription(registration)));
        } catch {
          subscribed = false;
        }
      }
      return pushCapabilityState({
        secureContext,
        pushSupported,
        serviceWorkerSupported,
        permission,
        subscribed,
      });
    },
    // 登出/换账号时的隐私退订：先于会话吊销调用（需要 Bearer），
    // 静默失败不阻塞登出流程。
    async disableSilently() {
      const token = typeof getSessionToken === "function" ? getSessionToken() : null;
      const gatewayUrl = getGatewayUrl();
      try {
        const registration = await navigatorRef.serviceWorker?.getRegistration("./");
        const subscription = registration
          ? await registration.pushManager.getSubscription()
          : null;
        if (!subscription) return;
        if (gatewayUrl && token) {
          await fetch(`${gatewayUrl}/v1/push/unsubscribe`, {
            method: "POST",
            headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
            body: JSON.stringify({ endpoint: subscription.endpoint }),
          }).catch(() => {});
        }
        await subscription.unsubscribe();
      } catch {
        // 静默失败：退出流程优先
      }
    },

    async toggle() {
      let failure = null;
      const state = await pushClient.state();
      try {
        if (state === "subscribed") {
          await disable();
        } else {
          await enable();
        }
      } catch (error) {
        failure = error instanceof Error ? error.message : "推送开关失败";
      }
      await pushClient.refresh();
      // 失败原因要在 refresh 之后写，否则会被 paint 的常规标题覆盖
      if (failure) button.title = failure;
      return failure;
    },
    async init() {
      button.addEventListener("click", () => {
        void pushClient.toggle();
      });
      return pushClient.refresh();
    },
  };

  return pushClient;
}

// 页面接线：仅在配置了网关且存在推送按钮的聊天页生效。
export function initPushClient(context = {}) {
  const doc = context.document ?? document;
  const gatewayUrl = context.gatewayUrl;
  if (!gatewayUrl) return null;
  const anchor = doc.querySelector?.("[data-push-toggle]");
  if (!anchor) return null;
  const client = createPushClient({
    document: doc,
    navigatorRef: context.navigatorRef ?? navigator,
    windowRef: context.windowRef ?? window,
    gatewayUrl,
    getSessionToken: context.getSessionToken ?? (() => null),
  });
  anchor.replaceWith(client.element);
  void client.init();
  return client;
}
