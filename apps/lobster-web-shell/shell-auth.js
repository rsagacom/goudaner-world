import { safeLocalStorageGet, safeLocalStorageSet, translateDeliveryMode } from "./shell-shared.js";

export function createAuthController(initialElMap = {}, initialCbs = {}) {
// --- module-scoped state ---
let _authSession = {
  challengeId: null,
  maskedEmail: null,
  expiresAtMs: null,
  deliveryMode: null,
};
let _sessionToken = null;
let _gatewayAuthFailure = false;

// --- cached DOM refs ---
let _els = {};

// --- callbacks to app.js ---
let _callbacks = {
  postJson: null,
  postAuthenticated: null,
  refreshFromGateway: null,
  persistIdentity: null,
  onGatewayAuthFailure: null,
  userProjection: null,
  gatewayUrl: null,
  desiredResidentId: null,
};

/**
 * Initialize auth module with DOM element references and required callbacks.
 * @param {object} elMap - DOM element references
 * @param {object} cbs   - Required callbacks { postJson, postAuthenticated, refreshFromGateway, persistIdentity, userProjection, gatewayUrl }
 */
function initAuth(elMap, cbs) {
  _els = {
    statusEl: elMap.statusEl || null,
    requestFormEl: elMap.requestFormEl || null,
    deliverySelectEl: elMap.deliverySelectEl || null,
    residentInputEl: elMap.residentInputEl || null,
    nicknameInputEl: elMap.nicknameInputEl || null,
    emailInputEl: elMap.emailInputEl || null,
    mobileInputEl: elMap.mobileInputEl || null,
    deviceInputEl: elMap.deviceInputEl || null,
    verifyFormEl: elMap.verifyFormEl || null,
    challengeInputEl: elMap.challengeInputEl || null,
    codeInputEl: elMap.codeInputEl || null,
    loginCardEl: elMap.loginCardEl || null,
    loginOverlayEl: elMap.loginOverlayEl || null,
    hudLoginToggleEl: elMap.hudLoginToggleEl || null,
  };
  _callbacks = {
    postJson: cbs.postJson || null,
    postAuthenticated: cbs.postAuthenticated || null,
    refreshFromGateway: cbs.refreshFromGateway || null,
    persistIdentity: cbs.persistIdentity || null,
    onGatewayAuthFailure: cbs.onGatewayAuthFailure || null,
    userProjection: cbs.userProjection || null,
    gatewayUrl: cbs.gatewayUrl || null,
    desiredResidentId: cbs.desiredResidentId || null,
  };
  if (_els.deliverySelectEl) {
    _els.deliverySelectEl.addEventListener("change", _onDeliveryModeChange);
  }
  _applyDeliveryInputVisibility();
}

function _onDeliveryModeChange() {
  _applyDeliveryInputVisibility();
  persistAuthDraft();
}

function _applyDeliveryInputVisibility() {
  const mode = _els.deliverySelectEl?.value || "email";
  const isMobile = mode === "mobile";
  if (_els.emailInputEl) {
    _els.emailInputEl.placeholder = isMobile ? "邮箱/可选反滥用" : "接收验证码的邮箱";
    _els.emailInputEl.required = !isMobile;
  }
  if (_els.mobileInputEl) {
    _els.mobileInputEl.placeholder = isMobile ? "手机号码（必填）" : "手机号/可选反滥用";
    _els.mobileInputEl.required = isMobile;
  }
}

// --- state accessors ---
function getSessionToken() {
  return _sessionToken;
}

function getAuthSession() {
  return _authSession;
}

function clearSession() {
  _sessionToken = null;
  safeLocalStorageSet("lobster-session-token", "");
}

function setSessionToken(token) {
  _sessionToken = token || null;
  if (token) _gatewayAuthFailure = false;
  safeLocalStorageSet("lobster-session-token", token || "");
}

async function logout() {
  const token = _sessionToken;
  let serverError = null;

  if (token) {
    if (typeof _callbacks.postAuthenticated !== "function") {
      serverError = new Error("authenticated logout is not configured");
    } else {
      try {
        await _callbacks.postAuthenticated("/v1/auth/logout", {}, token);
      } catch (error) {
        // Always clear the browser session, but do not claim that the server
        // session was revoked when the network call failed.
        serverError = error;
      }
    }
  }

  clearSession();
  _gatewayAuthFailure = false;
  _authSession = {
    challengeId: null,
    maskedEmail: null,
    expiresAtMs: null,
    deliveryMode: null,
  };
  if (_els.challengeInputEl) _els.challengeInputEl.value = "";
  if (_els.codeInputEl) _els.codeInputEl.value = "";
  if (_els.residentInputEl) _els.residentInputEl.value = "";
  persistAuthDraft();
  if (_callbacks.persistIdentity) {
    _callbacks.persistIdentity("访客");
  }
  if (_callbacks.refreshFromGateway) {
    await _callbacks.refreshFromGateway();
  }
  setAuthStatus(
    serverError ? "已退出本地登录，网关退出待重试" : "已退出登录",
    Boolean(serverError),
  );
  updateAuthFormState();
  return { serverLogout: !serverError, error: serverError };
}

// --- auth UI helpers ---
function setAuthStatus(message, isError = false) {
  if (!_els.statusEl) return;
  _els.statusEl.textContent = `登录状态：${message}`;
  _els.statusEl.classList.toggle("notice-pending", isError);
}

function currentDesiredResidentId() {
  const value = _els.residentInputEl?.value?.trim();
  return value || undefined;
}

function residentGatewayLoginRequired(
  userProjection,
  gatewayUrl,
  senderIdentity,
  sessionToken = null,
  allowSyntheticIdentity = false,
) {
  const isVisitor = !senderIdentity || String(senderIdentity).trim() === "访客" || !String(senderIdentity).trim();
  return Boolean(
    userProjection &&
    gatewayUrl &&
    !allowSyntheticIdentity &&
    (!sessionToken || isVisitor),
  );
}

function updateResidentLoginSurface(userProjection, gatewayUrl, senderIdentity, dismissed, authenticated = false) {
  if (!_els.loginCardEl) return;
  const needsLogin = Boolean(userProjection && gatewayUrl &&
    (!senderIdentity || String(senderIdentity).trim() === "访客" || !String(senderIdentity).trim()));
  const showOverlay = needsLogin && !dismissed;
  const signedIn = Boolean(authenticated && senderIdentity && String(senderIdentity).trim() !== "访客");

  _els.loginCardEl.classList.toggle("shell-hidden", !needsLogin);
  _els.loginCardEl.dataset.loginState = needsLogin ? "visitor" : "signed-in";

  if (_els.loginOverlayEl) {
    _els.loginOverlayEl.classList.toggle("shell-hidden", !showOverlay);
    _els.loginOverlayEl.setAttribute("aria-hidden", !showOverlay ? "true" : "false");
  }
  if (_els.hudLoginToggleEl) {
    _els.hudLoginToggleEl.classList.toggle("shell-hidden", !(signedIn || (needsLogin && dismissed)));
    _els.hudLoginToggleEl.textContent = signedIn ? "退出登录" : "登录";
    _els.hudLoginToggleEl.setAttribute("aria-label", signedIn ? "退出登录" : "打开登录窗口");
  }
  if (needsLogin && _els.statusEl && !_authSession.challengeId && !_gatewayAuthFailure) {
    setAuthStatus("访客模式 · 请登录后发送");
  }
}

function updateAuthFormState() {
  const gatewayUrl = typeof _callbacks.gatewayUrl === "function" ? _callbacks.gatewayUrl() : null;
  const enabled = Boolean(gatewayUrl);
  const verifyStep = Boolean(_authSession.challengeId);
  for (const element of [
    _els.deliverySelectEl,
    _els.nicknameInputEl,
    _els.emailInputEl,
    _els.mobileInputEl,
    _els.deviceInputEl,
  ]) {
    if (!element) continue;
    element.disabled = !enabled || verifyStep;
  }
  for (const element of [_els.challengeInputEl, _els.codeInputEl]) {
    if (!element) continue;
    element.disabled = !enabled || !verifyStep;
  }
  const authRequestButton = _els.requestFormEl?.querySelector("button");
  if (authRequestButton) authRequestButton.disabled = !enabled || verifyStep;
  const authVerifyButton = _els.verifyFormEl?.querySelector("button");
  if (authVerifyButton) authVerifyButton.disabled = !enabled || !verifyStep;
  _els.requestFormEl?.classList.toggle("is-auth-verify-step", verifyStep);
  _els.verifyFormEl?.classList.toggle("shell-hidden", !verifyStep);
  if (_els.requestFormEl) {
    _els.requestFormEl.dataset.authStep = verifyStep ? "verify" : "request";
  }
  if (_els.verifyFormEl) {
    _els.verifyFormEl.dataset.authStep = verifyStep ? "verify" : "request";
  }
}

// --- persistence ---
function loadAuthDraft() {
  const email = safeLocalStorageGet("lobster-auth-email");
  const mobile = safeLocalStorageGet("lobster-auth-mobile");
  const nickname = safeLocalStorageGet("lobster-auth-nickname");
  const resident = safeLocalStorageGet("lobster-auth-resident-id");
  const challengeId = safeLocalStorageGet("lobster-auth-challenge-id");
  const maskedEmail = safeLocalStorageGet("lobster-auth-masked-email");
  const deliveryMode = safeLocalStorageGet("lobster-auth-delivery-mode");
  const expiresAtMsRaw = safeLocalStorageGet("lobster-auth-expires-at-ms");
  const expiresAtMs = expiresAtMsRaw ? Number(expiresAtMsRaw) : null;
  const savedSessionToken = safeLocalStorageGet("lobster-session-token");
  _sessionToken = savedSessionToken || null;
  if (_els.emailInputEl && email) _els.emailInputEl.value = email;
  if (_els.mobileInputEl && mobile) _els.mobileInputEl.value = mobile;
  if (_els.residentInputEl && resident) _els.residentInputEl.value = resident;
  if (_els.nicknameInputEl && nickname) _els.nicknameInputEl.value = nickname;
  if (_els.challengeInputEl && challengeId) _els.challengeInputEl.value = challengeId;
  _authSession = {
    challengeId: challengeId || null,
    maskedEmail: maskedEmail || null,
    expiresAtMs: Number.isFinite(expiresAtMs) ? expiresAtMs : null,
    deliveryMode: deliveryMode || null,
  };
}

function persistAuthDraft() {
  safeLocalStorageSet("lobster-auth-resident-id", _els.residentInputEl?.value?.trim() || "");
  safeLocalStorageSet("lobster-auth-nickname", _els.nicknameInputEl?.value?.trim() || "");
  safeLocalStorageSet("lobster-auth-email", _els.emailInputEl?.value?.trim() || "");
  safeLocalStorageSet("lobster-auth-mobile", _els.mobileInputEl?.value?.trim() || "");
  safeLocalStorageSet("lobster-auth-challenge-id", _authSession.challengeId || "");
  safeLocalStorageSet("lobster-auth-masked-email", _authSession.maskedEmail || "");
  safeLocalStorageSet("lobster-auth-delivery-mode", _authSession.deliveryMode || "");
  safeLocalStorageSet(
    "lobster-auth-expires-at-ms",
    _authSession.expiresAtMs ? String(_authSession.expiresAtMs) : "",
  );
}

// --- gateway auth failure ---
function handleGatewayAuthFailure(status) {
  if (status !== 401 && status !== 403) return false;
  _gatewayAuthFailure = true;
  _sessionToken = null;
  safeLocalStorageSet("lobster-session-token", "");
  _authSession = {
    challengeId: null,
    maskedEmail: null,
    expiresAtMs: null,
    deliveryMode: null,
  };
  if (_els.challengeInputEl) _els.challengeInputEl.value = "";
  if (_els.codeInputEl) _els.codeInputEl.value = "";
  try {
    _callbacks.onGatewayAuthFailure?.(status);
  } catch {
    // Auth invalidation must still complete if a surface-specific UI callback fails.
  }
  setAuthStatus("登录已失效，请重新登录", true);
  updateAuthFormState();
  return true;
}

function hasGatewayAuthFailure() {
  return _gatewayAuthFailure;
}

// --- OTP flow ---
async function requestEmailOtp() {
  _gatewayAuthFailure = false;
  const deliveryMode = _els.deliverySelectEl?.value || "email";
  if (deliveryMode === "mobile") {
    return requestMobileOtp();
  }
  if (deliveryMode === "device") {
    setAuthStatus("设备验证将在后续版本支持", true);
    return;
  }
  const email = _els.emailInputEl?.value?.trim() || "";
  const mobile = _els.mobileInputEl?.value?.trim() || "";
  const devicePhysicalAddress = _els.deviceInputEl?.value?.trim() || "";
  if (!email) {
    setAuthStatus("请填写邮箱地址", true);
    return;
  }
  setAuthStatus("正在检查注册句柄");
  const preflightPayload = { email };
  if (mobile) preflightPayload.mobile = mobile;
  if (devicePhysicalAddress) preflightPayload.device_physical_address = devicePhysicalAddress;
  const preflight = await _callbacks.postJson("/v1/auth/preflight", preflightPayload);
  if (!preflight.allowed) {
    setAuthStatus(preflight.blocked_reasons.join(" · ") || "认证预检未通过", true);
    return;
  }
  const nickname = _els.nicknameInputEl?.value?.trim() || undefined;
  setAuthStatus(`正在为 ${preflight.normalized_email || email} 申请邮箱验证码`);
  const requestPayload = { email };
  if (mobile) requestPayload.mobile = mobile;
  if (devicePhysicalAddress) requestPayload.device_physical_address = devicePhysicalAddress;
  const desiredResidentId = _callbacks.desiredResidentId ? _callbacks.desiredResidentId() : undefined;
  if (desiredResidentId) requestPayload.resident_id = desiredResidentId;
  if (nickname) requestPayload.nickname = nickname;
  const response = await _callbacks.postJson("/v1/auth/email-otp/request", requestPayload);
  _authSession = {
    challengeId: response.challenge_id,
    maskedEmail: response.masked_email,
    expiresAtMs: response.expires_at_ms,
    deliveryMode: response.delivery_mode,
  };
  if (_els.challengeInputEl) _els.challengeInputEl.value = response.challenge_id;
  if (response.dev_code && _els.codeInputEl) {
    _els.codeInputEl.value = response.dev_code;
  }
  persistAuthDraft();
  const expiresAt = new Date(response.expires_at_ms).toLocaleTimeString();
  const deliveryNote = response.dev_code
    ? `开发验证码已预填 · ${expiresAt} 前有效`
    : `${translateDeliveryMode(response.delivery_mode)} · ${expiresAt} 前有效`;
  setAuthStatus(`邮箱验证码已发往 ${response.masked_email} · ${deliveryNote}`);
}

async function requestMobileOtp() {
  _gatewayAuthFailure = false;
  const mobile = _els.mobileInputEl?.value?.trim() || "";
  const email = _els.emailInputEl?.value?.trim() || "";
  const devicePhysicalAddress = _els.deviceInputEl?.value?.trim() || "";
  if (!mobile) {
    setAuthStatus("请填写手机号码", true);
    return;
  }
  setAuthStatus("正在检查注册句柄");
  const preflightPayload = { mobile };
  if (email) preflightPayload.email = email;
  if (devicePhysicalAddress) preflightPayload.device_physical_address = devicePhysicalAddress;
  const preflight = await _callbacks.postJson("/v1/auth/preflight", preflightPayload);
  if (!preflight.allowed) {
    setAuthStatus(preflight.blocked_reasons.join(" · ") || "认证预检未通过", true);
    return;
  }
  const nickname = _els.nicknameInputEl?.value?.trim() || undefined;
  setAuthStatus(`正在为 ${preflight.normalized_mobile || mobile} 申请手机验证码`);
  const requestPayload = { mobile };
  if (email) requestPayload.email = email;
  if (devicePhysicalAddress) requestPayload.device_physical_address = devicePhysicalAddress;
  const desiredResidentId = _callbacks.desiredResidentId ? _callbacks.desiredResidentId() : undefined;
  if (desiredResidentId) requestPayload.resident_id = desiredResidentId;
  if (nickname) requestPayload.nickname = nickname;
  const response = await _callbacks.postJson("/v1/auth/mobile-otp/request", requestPayload);
  _authSession = {
    challengeId: response.challenge_id,
    maskedEmail: response.masked_mobile,
    expiresAtMs: response.expires_at_ms,
    deliveryMode: response.delivery_mode,
  };
  if (_els.challengeInputEl) _els.challengeInputEl.value = response.challenge_id;
  if (response.dev_code && _els.codeInputEl) {
    _els.codeInputEl.value = response.dev_code;
  }
  persistAuthDraft();
  const expiresAt = new Date(response.expires_at_ms).toLocaleTimeString();
  const deliveryNote = response.dev_code
    ? `开发验证码已预填 · ${expiresAt} 前有效`
    : `${translateDeliveryMode(response.delivery_mode)} · ${expiresAt} 前有效`;
  setAuthStatus(`手机验证码已发往 ${response.masked_mobile} · ${deliveryNote}`);
}

function enterDemoVerifyStep(maskedEmail) {
  _authSession = {
    challengeId: "demo-challenge",
    maskedEmail: maskedEmail || "demo@example.com",
    expiresAtMs: Date.now() + 300000,
    deliveryMode: "email",
  };
  persistAuthDraft();
  updateAuthFormState();
}

async function verifyEmailOtp() {
  const challengeId = (_authSession.challengeId || _els.challengeInputEl?.value || "").trim();
  const code = _els.codeInputEl?.value?.trim() || "";
  if (!challengeId) {
    setAuthStatus("请先获取验证码", true);
    return;
  }
  if (!code) {
    setAuthStatus("请填写验证码", true);
    return;
  }
  if (challengeId === "demo-challenge") {
    _gatewayAuthFailure = false;
    const residentId = _callbacks.desiredResidentId ? _callbacks.desiredResidentId() : "demo-resident";
    if (_callbacks.persistIdentity) {
      _callbacks.persistIdentity(residentId);
    }
    if (_els.residentInputEl) _els.residentInputEl.value = residentId;
    _sessionToken = "demo-session-token";
    safeLocalStorageSet("lobster-session-token", _sessionToken);
    _authSession = {
      challengeId: null,
      maskedEmail: _authSession.maskedEmail || "",
      expiresAtMs: null,
      deliveryMode: null,
    };
    if (_els.challengeInputEl) _els.challengeInputEl.value = "";
    if (_els.codeInputEl) _els.codeInputEl.value = "";
    persistAuthDraft();
    if (_callbacks.refreshFromGateway) await _callbacks.refreshFromGateway();
    setAuthStatus(`已登录为 ${residentId}`);
    return;
  }
  const isMobile = challengeId.startsWith("mobile-otp:");
  const endpoint = isMobile ? "/v1/auth/mobile-otp/verify" : "/v1/auth/email-otp/verify";
  const verifyLabel = isMobile ? "手机" : "邮箱";
  setAuthStatus(`正在校验${verifyLabel}验证码`);
  const response = await _callbacks.postJson(endpoint, {
    challenge_id: challengeId,
    code,
    resident_id: _callbacks.desiredResidentId ? _callbacks.desiredResidentId() : undefined,
  });
  if (_callbacks.persistIdentity) {
    _callbacks.persistIdentity(response.resident_id);
  }
  if (_els.residentInputEl) _els.residentInputEl.value = response.resident_id;
  _sessionToken = response.session_token || null;
  _gatewayAuthFailure = false;
  safeLocalStorageSet("lobster-session-token", _sessionToken || "");
  const masked = response.mobile_masked || response.email_masked || "";
  _authSession = {
    challengeId: null,
    maskedEmail: masked,
    expiresAtMs: null,
    deliveryMode: null,
  };
  if (_els.challengeInputEl) _els.challengeInputEl.value = "";
  if (_els.codeInputEl) _els.codeInputEl.value = "";
  persistAuthDraft();
  const displayName = response.nickname || response.resident_id;
  await _callbacks.refreshFromGateway();
  setAuthStatus(`已登录为 ${displayName} · ${masked}`);
}

async function updateMyNickname(nickname) {
  if (!_sessionToken) {
    setAuthStatus("请先登录", true);
    return null;
  }
  try {
    const resp = await _callbacks.postJson("/v1/shell/nickname", { nickname: nickname || undefined });
    if (resp.ok) {
      setAuthStatus(nickname ? `显示名称已更新为 ${nickname}` : "显示名称已清除");
      if (_callbacks.refreshFromGateway) await _callbacks.refreshFromGateway();
      return resp.nickname;
    }
  } catch (e) {
    setAuthStatus("更新昵称请求失败", true);
  }
  return null;
}

  if (Object.keys(initialElMap).length || Object.keys(initialCbs).length) {
    initAuth(initialElMap, initialCbs);
  }

  return {
    clearSession,
    currentDesiredResidentId,
    enterDemoVerifyStep,
    getAuthSession,
    getSessionToken,
    handleGatewayAuthFailure,
    hasGatewayAuthFailure,
    initAuth,
    loadAuthDraft,
    persistAuthDraft,
    requestEmailOtp,
    logout,
    residentGatewayLoginRequired,
    setAuthStatus,
    setSessionToken,
    updateAuthFormState,
    updateMyNickname,
    updateResidentLoginSurface,
    verifyEmailOtp,
  };
}

const defaultAuthController = createAuthController();

export const initAuth = (...args) => defaultAuthController.initAuth(...args);
export const getSessionToken = (...args) => defaultAuthController.getSessionToken(...args);
export const getAuthSession = (...args) => defaultAuthController.getAuthSession(...args);
export const clearSession = (...args) => defaultAuthController.clearSession(...args);
export const setSessionToken = (...args) => defaultAuthController.setSessionToken(...args);
export const setAuthStatus = (...args) => defaultAuthController.setAuthStatus(...args);
export const currentDesiredResidentId = (...args) => defaultAuthController.currentDesiredResidentId(...args);
export const residentGatewayLoginRequired = (...args) => defaultAuthController.residentGatewayLoginRequired(...args);
export const updateResidentLoginSurface = (...args) => defaultAuthController.updateResidentLoginSurface(...args);
export const updateAuthFormState = (...args) => defaultAuthController.updateAuthFormState(...args);
export const loadAuthDraft = (...args) => defaultAuthController.loadAuthDraft(...args);
export const persistAuthDraft = (...args) => defaultAuthController.persistAuthDraft(...args);
export const handleGatewayAuthFailure = (...args) => defaultAuthController.handleGatewayAuthFailure(...args);
export const requestEmailOtp = (...args) => defaultAuthController.requestEmailOtp(...args);
export const logout = (...args) => defaultAuthController.logout(...args);
export const enterDemoVerifyStep = (...args) => defaultAuthController.enterDemoVerifyStep(...args);
export const verifyEmailOtp = (...args) => defaultAuthController.verifyEmailOtp(...args);
export const updateMyNickname = (...args) => defaultAuthController.updateMyNickname(...args);
