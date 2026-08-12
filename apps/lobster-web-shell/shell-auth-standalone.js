import { createAuthController } from "./shell-auth.js";
import { gatewayErrorMessage, localizedRuntimeError } from "./shell-errors.js";

function byId(id) {
  return document.getElementById(id);
}

function resolveGatewayUrl(gatewayUrl) {
  if (typeof gatewayUrl === "function") return gatewayUrl;
  const value = gatewayUrl ?? new URLSearchParams(window.location.search).get("gateway") ?? "";
  return () => value;
}

async function postGatewayJson(gatewayUrl, path, body) {
  if (!gatewayUrl) throw new Error("gateway not connected");
  const response = await fetch(gatewayUrl.replace(/\/+$/, "") + path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = {};
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = {};
    }
  }
  if (!response.ok) {
    throw new Error(gatewayErrorMessage(parsed, text, response.status));
  }
  return parsed;
}

async function postGatewayJsonAuthenticated(gatewayUrl, path, body, token) {
  if (!gatewayUrl) throw new Error("gateway not connected");
  const response = await fetch(gatewayUrl.replace(/\/+$/, "") + path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(body ?? {}),
  });
  const text = await response.text();
  let parsed = {};
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = {};
    }
  }
  if (!response.ok) {
    throw new Error(gatewayErrorMessage(parsed, text, response.status));
  }
  return parsed;
}

function openLoginOverlay(els) {
  els.loginOverlayEl?.classList.remove("shell-hidden");
  els.loginOverlayEl?.setAttribute("aria-hidden", "false");
}

function closeLoginOverlay(els) {
  els.loginOverlayEl?.classList.add("shell-hidden");
  els.loginOverlayEl?.setAttribute("aria-hidden", "true");
}

export function initStandaloneAuthSurface(options = {}) {
  const gatewayUrl = resolveGatewayUrl(options.gatewayUrl);
  const els = {
    statusEl: byId("auth-status"),
    requestFormEl: byId("auth-request-form"),
    deliverySelectEl: byId("auth-delivery-select"),
    residentInputEl: byId("auth-resident-input"),
    nicknameInputEl: byId("auth-nickname-input"),
    emailInputEl: byId("auth-email-input"),
    mobileInputEl: byId("auth-mobile-input"),
    deviceInputEl: byId("auth-device-input"),
    verifyFormEl: byId("auth-verify-form"),
    challengeInputEl: byId("auth-challenge-input"),
    codeInputEl: byId("auth-code-input"),
    loginCardEl: byId("resident-login-card"),
    loginOverlayEl: byId("resident-login-overlay"),
    hudLoginToggleEl: byId("hud-login-toggle"),
  };

  const authController = createAuthController(els, {
    gatewayUrl,
    postJson: options.postJson || ((path, body) => postGatewayJson(gatewayUrl(), path, body)),
    postAuthenticated: options.postAuthenticated || ((path, body, token) =>
      postGatewayJsonAuthenticated(gatewayUrl(), path, body, token)),
    refreshFromGateway: options.refreshFromGateway || (async () => {}),
    onGatewayAuthFailure: (status) => {
      try {
        localStorage.setItem(options.identityStorageKey || "lobster-identity", "访客");
      } catch {}
      if (els.residentInputEl) els.residentInputEl.value = "访客";
      if (els.hudLoginToggleEl) {
        els.hudLoginToggleEl.classList.toggle("shell-hidden", !gatewayUrl());
        els.hudLoginToggleEl.disabled = false;
        els.hudLoginToggleEl.textContent = "登录";
        els.hudLoginToggleEl.setAttribute("aria-label", "打开登录窗口");
      }
      options.onAuthFailure?.(status);
    },
    persistIdentity: (id) => {
      try {
        localStorage.setItem(options.identityStorageKey || "lobster-identity", id);
      } catch {}
      const signedIn = Boolean(id && id !== "访客");
      if (els.hudLoginToggleEl) {
        els.hudLoginToggleEl.classList.toggle("shell-hidden", !signedIn && !gatewayUrl());
        els.hudLoginToggleEl.textContent = signedIn ? "退出登录" : "登录";
        els.hudLoginToggleEl.setAttribute("aria-label", signedIn ? "退出登录" : "打开登录窗口");
      }
      options.onIdentityChanged?.(id);
    },
    userProjection: options.userProjection || (() => null),
    desiredResidentId: options.desiredResidentId || (() => undefined),
  });
  const {
    loadAuthDraft,
    persistAuthDraft,
    requestEmailOtp,
    setAuthStatus,
    updateAuthFormState,
    verifyEmailOtp,
  } = authController;

  loadAuthDraft();
  updateAuthFormState();

  els.requestFormEl?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const button = els.requestFormEl.querySelector("button");
    if (button) button.disabled = true;
    persistAuthDraft();
    try {
      await requestEmailOtp();
    } catch (error) {
      setAuthStatus(localizedRuntimeError(error, "申请验证码失败"), true);
    } finally {
      updateAuthFormState();
    }
  });

  els.verifyFormEl?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const button = els.verifyFormEl.querySelector("button");
    if (button) button.disabled = true;
    persistAuthDraft();
    try {
      await verifyEmailOtp();
      closeLoginOverlay(els);
    } catch (error) {
      setAuthStatus(localizedRuntimeError(error, "验证码校验失败"), true);
    } finally {
      updateAuthFormState();
    }
  });

  els.hudLoginToggleEl?.addEventListener("click", () => {
    if (authController.getSessionToken()) {
      els.hudLoginToggleEl.disabled = true;
      authController.logout().finally(() => {
        els.hudLoginToggleEl.disabled = false;
        els.hudLoginToggleEl.textContent = "登录";
        els.hudLoginToggleEl.setAttribute("aria-label", "打开登录窗口");
      });
      return;
    }
    openLoginOverlay(els);
  });
  byId("resident-login-close")?.addEventListener("click", () => {
    closeLoginOverlay(els);
  });

  return {
    els,
    gatewayUrl,
    open: () => openLoginOverlay(els),
    close: () => closeLoginOverlay(els),
    refresh: updateAuthFormState,
    authController,
  };
}
