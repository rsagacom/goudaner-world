// shell-install-hint.js — PWA 加桌引导（manifest 最小版配套，无 service worker）。
// 纯 DOM 模块 + 可单测决策函数，不依赖 app.js 全局。
// 视觉规范:dark-on-dark,禁大块金色/cream。
//
// 决策合同（installHintState）:
//   - 已安装（standalone 显示模式或 iOS navigator.standalone）→ 不提示
//   - 用户点过关闭（dismissed）→ 不提示
//   - Chrome/Android 触发 beforeinstallprompt → 显示"安装"按钮（点了才 prompt()）
//   - iOS-like（iPhone/iPad 触屏，无 beforeinstallprompt 机制）→ 显示"分享 → 添加到主屏幕"指引
//   - 其余桌面环境 → 不提示（桌面浏览器装 PWA 收益低，保持克制）

export const INSTALL_HINT_DISMISS_KEY = "lobster-install-hint-dismissed";

export function installHintState({
  displayStandalone = false,
  iosStandalone = false,
  iosLike = false,
  dismissed = false,
  hasInstallPrompt = false,
} = {}) {
  if (displayStandalone || iosStandalone) return "hidden";
  if (dismissed) return "hidden";
  if (hasInstallPrompt) return "prompt";
  if (iosLike) return "ios";
  return "hidden";
}

export function detectIosLike(navigatorRef = navigator) {
  const ua = typeof navigatorRef.userAgent === "string" ? navigatorRef.userAgent : "";
  const isAppleMobile = /iphone|ipad|ipod/i.test(ua);
  const isIpadosMasquerade = /macintosh/i.test(ua) && navigatorRef.maxTouchPoints > 1;
  return isAppleMobile || isIpadosMasquerade;
}

export function isStandaloneDisplay(matchMediaRef = window.matchMedia, navigatorRef = navigator) {
  try {
    if (typeof matchMediaRef === "function" && matchMediaRef("(display-mode: standalone)").matches) {
      return true;
    }
  } catch {
    // 非 DOM 环境按未安装处理
  }
  return navigatorRef.standalone === true;
}

export function createInstallHint({
  document: doc = document,
  storage = window.localStorage,
  onInstall = () => {},
} = {}) {
  const chip = doc.createElement("div");
  chip.className = "install-hint-chip";
  chip.hidden = true;

  const copy = doc.createElement("span");
  copy.className = "install-hint-copy";

  const action = doc.createElement("button");
  action.type = "button";
  action.className = "install-hint-action";
  action.hidden = true;

  const dismiss = doc.createElement("button");
  dismiss.type = "button";
  dismiss.className = "install-hint-dismiss";
  dismiss.textContent = "×";
  dismiss.setAttribute("aria-label", "关闭加桌引导");

  chip.appendChild(copy);
  chip.appendChild(action);
  chip.appendChild(dismiss);

  let deferredPrompt = null;

  function show(state) {
    if (state === "prompt") {
      copy.textContent = "把「狗蛋儿」装到桌面，秒开不占内存";
      action.textContent = "安装";
      action.hidden = false;
    } else {
      copy.textContent = "iOS：分享 → 添加到主屏幕，像 App 一样秒开";
      action.hidden = true;
    }
    chip.hidden = false;
  }

  function readDismissed() {
    try {
      return storage.getItem(INSTALL_HINT_DISMISS_KEY) === "1";
    } catch {
      return false;
    }
  }

  function init({ displayStandalone = false, iosLike = false, hasInstallPrompt = false } = {}) {
    dismiss.addEventListener("click", () => {
      chip.hidden = true;
      try {
        storage.setItem(INSTALL_HINT_DISMISS_KEY, "1");
      } catch {
        // 隐私模式等存储失败时本次会话内仍然隐藏
      }
    });
    action.addEventListener("click", () => {
      if (!deferredPrompt) return;
      deferredPrompt.prompt();
      deferredPrompt = null;
      chip.hidden = true;
      onInstall();
    });
    const state = installHintState({ displayStandalone, iosLike, dismissed: readDismissed(), hasInstallPrompt });
    if (state !== "hidden") show(state);
    return state;
  }

  function setDeferredPrompt(prompt) {
    deferredPrompt = prompt;
    if (installHintState({ displayStandalone: false, dismissed: readDismissed(), hasInstallPrompt: true }) === "prompt") {
      show("prompt");
    }
  }

  return {
    element: chip,
    init,
    setDeferredPrompt,
    action,
    dismiss,
    copy,
    isOpen: () => !chip.hidden,
  };
}

// 页面接线：只在聊天页（存在 #timeline）生效；返回实例便于单测。
export function initInstallHint(context = {}) {
  const doc = context.document ?? document;
  if (!doc.querySelector?.("#timeline")) return null;
  const storage = context.storage ?? window.localStorage;
  const hint = createInstallHint({ document: doc, storage });
  (doc.body ?? doc).appendChild(hint.element);
  const started = hint.init({
    displayStandalone: context.displayStandalone ?? isStandaloneDisplay(),
    iosLike: context.iosLike ?? detectIosLike(),
  });
  if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
    window.addEventListener("beforeinstallprompt", (event) => {
      event.preventDefault?.();
      hint.setDeferredPrompt(event);
    });
  }
  return { hint, started };
}
