/* ============================================================
   admin-ds-runtime.test.mjs — admin-ds gateway 数据流运行时测试
   覆盖：
   - gateway 成功时 normalize 函数正确转换数据
   - gateway 失败时返回 null 不污染 mock
   - 无 gateway 时 fetchGatewayJson 直接返回 null
   - 写操作按钮在源文件中正确标记
   - 外部数据全部通过 textContent/DOM API 写入
   ============================================================ */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const serial = { concurrency: false };

async function readText(relPath) {
  return fs.readFile(new URL(relPath, import.meta.url), "utf8");
}

// Set up just enough globals to load admin-ds.js without crashing
function setupMinimalGlobals(gatewayUrl = null) {
  const storage = new Map();
  Object.defineProperty(globalThis, "navigator", { value: { userAgent: "node-test" }, writable: true, configurable: true });
  globalThis.document = {
    body: { dataset: {}, classList: { add() {}, remove() {}, toggle() {}, contains() {} }, querySelector() { return null; }, querySelectorAll() { return []; }, appendChild() {}, addEventListener() {}, removeEventListener() {} },
    getElementById(id) { return globalThis._dsElements?.[id] || null; },
    querySelector(s) { return null; },
    querySelectorAll() { return []; },
    createElement(tag) {
      const el = {
        tagName: tag.toUpperCase(), className: "", textContent: "", dataset: {},
        style: { cssText: "" }, children: [], disabled: false,
        classList: { _v: "", add(c) { this._v += " " + c; }, remove(c) { this._v = this._v.replace(c, "").trim(); }, toggle(c, on) { if (on === undefined) this._v = this._v.includes(c) ? this._v.replace(c, "").trim() : this._v + " " + c; else if (on) this._v += " " + c; else this._v = this._v.replace(c, "").trim(); }, contains(c) { return this._v.includes(c); } },
        setAttribute(n, v) { this[n] = v; },
        getAttribute(n) { return this[n] || null; },
        appendChild(c) { this.children.push(c); return c; },
        insertBefore(c) { this.children.unshift(c); return c; },
        removeChild(c) { return c; },
        querySelector() { return null; },
        querySelectorAll() { return []; },
        closest() { return null; },
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {},
      };
      return el;
    },
    createTextNode(text) { return { nodeType: 3, textContent: text }; },
    createDocumentFragment() { return { appendChild(c) { return c; }, children: [] }; },
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent() {},
    activeElement: { tagName: "BODY" },
  };
  // Store elements for lookup
  globalThis._dsElements = {};

  function el(id, opts = {}) {
    const e = globalThis.document.createElement(opts.tag || "div");
    e.id = id;
    if (opts.className) e.className = opts.className;
    if (opts.value !== undefined) e.value = opts.value;
    globalThis._dsElements[id] = e;
    return e;
  }

  // Create all elements admin-ds.js expects
  for (const id of [
    "statGateway", "statGatewaySub", "statOnlineResidents", "statOnlineSub",
    "statTodayMessages", "statMessageSub", "statPendingAlerts", "statAlertSub",
    "dsGatewayEndpoint", "dsGatewayConnection", "dsGatewayResident",
    "dsGatewayRoomCount", "dsGatewayMessageCount", "dsGatewayLastSync",
    "dsGatewayStatus", "dsOnlineCount", "dsAlertCount", "dashboardTime",
    "msgAuditBadge", "dsSidebar", "dsSidebarToggle", "dsSidebarOverlay",
    "logAlertBadge",
    "dsDetailPanel", "dsDetailTitle", "dsDetailBody", "dsDetailActions",
    "dsDetailClose", "dsContent", "dsAdminNotice",
  ]) { el(id); }

  for (const id of [
    "residentTableBody", "roomTableBody", "msgTableBody", "inviteTableBody", "logTableBody",
    "worldNoticeTableBody", "safetyAdvisoryTableBody", "safetyReportTableBody", "sanctionTableBody",
  ]) {
    el(id, { tag: "tbody" });
  }
  for (const id of [
    "residentSearch", "residentStatusFilter", "residentRoleFilter",
    "roomSearch", "roomTypeFilter", "msgSearch", "msgRoomFilter", "msgStatusFilter",
    "logSearch", "logLevelFilter", "logTypeFilter",
  ]) { el(id, { tag: "input", value: "" }); }

  for (const id of ["mod-dashboard", "mod-residents", "mod-rooms", "mod-messages", "mod-permissions", "mod-sysconfig", "mod-logs"]) {
    el(id, { className: "ds-module" });
  }

  for (const action of ["export-residents", "create-resident", "batch-approve-messages", "refresh-messages", "create-permission-group", "generate-invite", "export-logs", "clear-processed-logs"]) {
    const btn = el("btn-" + action, { tag: "button", className: "ds-btn" });
    btn.dataset.adminAction = action;
  }

  for (const nav of ["dashboard", "residents", "rooms", "messages", "permissions", "sysconfig", "logs"]) {
    const navEl = el("nav-" + nav, { tag: "button" });
    navEl.dataset.module = nav;
  }

  const search = gatewayUrl ? `?gateway=${gatewayUrl}` : "";
  globalThis.window = {
    location: { href: `http://127.0.0.1:18081/admin-ds.html${search}`, search, protocol: "http:", origin: "http://127.0.0.1:18081" },
    localStorage: { getItem(k) { return storage.get(k) || null; }, setItem(k, v) { storage.set(k, String(v)); }, removeItem(k) { storage.delete(k); } },
    addEventListener() {}, removeEventListener() {}, dispatchEvent() {},
    setTimeout(fn, d) { if (typeof fn === "function") fn(); return 1; },
    clearTimeout() {}, setInterval() { return 1; }, clearInterval() {},
    requestAnimationFrame(fn) { if (typeof fn === "function") fn(); return 1; },
    cancelAnimationFrame() {},
    innerWidth: 1280,
    matchMedia() { return { matches: false, addEventListener() {}, removeEventListener() {} }; },
  };
  globalThis.localStorage = globalThis.window.localStorage;
  globalThis.requestAnimationFrame = globalThis.window.requestAnimationFrame;
  globalThis.cancelAnimationFrame = globalThis.window.cancelAnimationFrame;
  globalThis.setTimeout = globalThis.window.setTimeout;
  globalThis.clearTimeout = globalThis.window.clearTimeout;
  globalThis.setInterval = globalThis.window.setInterval;
  globalThis.clearInterval = globalThis.window.clearInterval;
  globalThis.HTMLElement = function() {};
  globalThis.Element = function() {};
  globalThis.Event = function(type) { this.type = type; };
  globalThis.CustomEvent = function(type, opts) { this.type = type; this.detail = opts?.detail; };
  globalThis.URLSearchParams = URLSearchParams;
}

// Load admin-ds.js with patched exports for testing normalize functions
async function loadAdminDsWithExports(opts = {}) {
  const dataJs = await readText("../admin-ds-data.js");
  const mainJs = await readText("../admin-ds.js");

  // Patch: expose normalize functions before bindStaticAdminActions
  const patchedJs = mainJs.replace(
    /function bindStaticAdminActions/,
    "window.__adminTest = window.__adminTest || {};\n" +
    "window.__adminTest.normalizeGatewayResidents = normalizeGatewayResidents;\n" +
    "window.__adminTest.normalizeGatewayRooms = normalizeGatewayRooms;\n" +
    "window.__adminTest.normalizeGatewayMessages = normalizeGatewayMessages;\n" +
    "window.__adminTest.updateDashboardSummary = updateDashboardSummary;\n" +
    "window.__adminTest.updateAlertCounts = updateAlertCounts;\n" +
    "window.__adminTest.fetchGatewayJson = fetchGatewayJson;\n" +
    "window.__adminTest.fetchGatewayJsonPost = fetchGatewayJsonPost;\n" +
    "window.__adminTest.resolveGatewayUrl = resolveGatewayUrl;\n" +
    "window.__adminTest.markUnavailableButton = markUnavailableButton;\n" +
    "window.__adminTest.showAdminNotice = showAdminNotice;\n" +
    "window.__adminTest.banResident = banResident;\n" +
    "window.__adminTest.unbanResident = unbanResident;\n" +
    "window.__adminTest.unsanctionResident = unsanctionResident;\n" +
    "window.__adminTest.freezeRoom = freezeRoom;\n" +
    "window.__adminTest.unfreezeRoom = unfreezeRoom;\n" +
    "window.__adminTest.loadInviteCodes = loadInviteCodes;\n" +
    "window.__adminTest.loadAuditLog = loadAuditLog;\n" +
    "window.__adminTest.loadGatewayAdminData = loadGatewayAdminData;\n" +
    "window.__adminTest.loadPermissionGroups = loadPermissionGroups;\n" +
    "window.__adminTest.permissionGroupItems = permissionGroupItems;\n" +
    "window.__adminTest.loadWorldNotices = loadWorldNotices;\n" +
    "window.__adminTest.loadSafetyData = loadSafetyData;\n" +
    "window.__adminTest.getInviteCodes = () => inviteCodes;\n" +
    "window.__adminTest.getLogs = () => logs;\n" +
    "window.__adminTest.getWorldNotices = () => worldNotices;\n" +
    "window.__adminTest.getSafetyAdvisories = () => safetyAdvisories;\n" +
    "window.__adminTest.getSafetyReports = () => safetyReports;\n" +
    "window.__adminTest.getResidentSanctions = () => residentSanctions;\n" +
    "window.__adminTest.getResidents = () => residents;\n" +
    "window.__adminTest.getRooms = () => rooms;\n" +
    "window.__adminTest.getMessages = () => messages;\n" +
    "window.__adminTest.summarizeBatchResults = summarizeBatchResults;\n" +
    "function bindStaticAdminActions"
  );

  const tmpDir = path.join((import.meta.dirname || path.dirname(new URL(import.meta.url).pathname)), "..", ".tmp");
  await fs.mkdir(tmpDir, { recursive: true });
  const tmpPath = path.join(tmpDir, `ads-exp-${Date.now()}-${Math.random().toString(16).slice(2)}.js`);
  await fs.writeFile(tmpPath, dataJs + "\n" + patchedJs, "utf8");

  try {
    if (opts.fetchMock) {
      globalThis.fetch = opts.fetchMock;
    } else {
      globalThis.fetch = async () => ({ ok: false, status: 500, json: async () => ({}), text: async () => "error" });
    }
    await import(`${pathToFileURL(tmpPath).href}?t=${Date.now()}`);
  } finally {
    await fs.unlink(tmpPath).catch(() => {});
  }

  return globalThis.window?.__adminTest || {};
}

// ---- Tests ----

test("admin-ds runtime: normalizeGatewayResidents 正确转换 gateway 数据", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();

  assert.equal(typeof api.normalizeGatewayResidents, "function", "normalizeGatewayResidents 应暴露为函数");

  const payload = [
    { resident_id: "alice", nick: "爱丽丝", roles: ["Resident"], online: true, last_seen_at_ms: Date.now() },
    { resident_id: "bob", nick: "鲍勃", roles: ["Admin"], online: false, last_seen_at_ms: Date.now() - 3600000 },
  ];
  const result = api.normalizeGatewayResidents(payload);
  assert.ok(Array.isArray(result), "应返回数组");
  assert.equal(result.length, 2, "应返回 2 条记录");

  const first = result[0];
  assert.equal(first.id, "alice", "id 应为 resident_id");
  assert.equal(first.nick, "alice", "nick 映射为 id");
  assert.equal(first.status, "online", "online=true -> status=online");

  // 安全验证：不应包含 HTML 标签或实体
  assert.equal(first.nick.includes("<"), false, "数据不应包含 HTML 标签");
  assert.equal(first.nick.includes(">"), false, "数据不应包含 HTML 标签");
});

test("admin-ds runtime: 注册审计字段映射到居民后台", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  const result = api.normalizeGatewayResidents([{
    resident_id: "registered-only",
    nickname: "新居民",
    email_masked: "r***@example.com",
    registration_state: "suspended",
    roles: [],
    active_cities: [],
    online: false,
    created_at_ms: 1710000000000,
    verified_at_ms: 1710000001000,
    last_login_at_ms: 1710000002000
  }]);

  assert.equal(result[0].email, "r***@example.com");
  assert.equal(result[0].status, "banned");
  assert.equal(result[0].registrationState, "suspended");
  assert.equal(result[0].createdAtMs, 1710000000000);
  assert.equal(result[0].verifiedAtMs, 1710000001000);
  assert.equal(result[0].lastLoginAtMs, 1710000002000);
});

test("admin-ds runtime: normalizeGatewayRooms 正确转换 shell state", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();

  assert.equal(typeof api.normalizeGatewayRooms, "function", "normalizeGatewayRooms 应暴露为函数");

  const shellState = {
    rooms: [
      { id: "room:world:lobby", title: "主城大厅", kind: "public", participant_count: 24 },
      { id: "dm:alice:bob", title: null, kind: "direct", peer_display: "鲍勃", participant_count: 2 },
    ],
  };
  const result = api.normalizeGatewayRooms(shellState);
  assert.ok(Array.isArray(result), "应返回数组");
  assert.equal(result.length, 2, "应返回 2 个房间");
  assert.ok(result[0].name && result[0].name.length > 0, "房间应有名称");
});

test("admin-ds runtime: normalizeGatewayMessages 正确转换 shell state", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();

  assert.equal(typeof api.normalizeGatewayMessages, "function", "normalizeGatewayMessages 应暴露为函数");

  const shellState = {
    rooms: [
      {
        id: "room:world:lobby",
        messages: [
          { message_id: "msg-1", sender: "爱丽丝", text: "大家好", created_at_ms: Date.now() },
          { message_id: "msg-2", sender: "鲍勃", text: "你好", created_at_ms: Date.now() },
        ],
      },
    ],
  };
  const result = api.normalizeGatewayMessages(shellState);
  assert.ok(Array.isArray(result), "应返回数组");
  assert.ok(result.length > 0, "应至少返回一条消息");
  const first = result[0];
  assert.ok(first.sender, "应有 sender");
  assert.ok(first.content !== undefined, "应有 content");
  assert.ok(first.room, "应有 room 上下文");
});

test("admin-ds runtime: fetchGatewayJson 在无 gateway 时返回 null", serial, async () => {
  setupMinimalGlobals(null);
  const api = await loadAdminDsWithExports();

  assert.equal(typeof api.fetchGatewayJson, "function", "fetchGatewayJson 应暴露为函数");

  const result = await api.fetchGatewayJson("/v1/residents");
  assert.equal(result, null, "无 gateway 时应返回 null");
});

test("admin-ds runtime: fetchGatewayJson 在 gateway 失败时返回 null", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:9999");
  globalThis.fetch = async () => ({ ok: false, status: 500, json: async () => ({}), text: async () => "error" });
  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });

  assert.equal(typeof api.fetchGatewayJson, "function", "fetchGatewayJson 应暴露为函数");

  const result = await api.fetchGatewayJson("/v1/residents");
  assert.equal(result, null, "gateway 失败时应返回 null 而非 crash");
});

test("admin-ds runtime: 401/403 进入共享认证失效回调并保留 deferred failure", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:9999");
  globalThis.fetch = async () => ({
    ok: false,
    status: 401,
    json: async () => { throw new Error("invalid json body"); },
    text: async () => '{"error":"invalid or expired session"}',
  });
  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });

  assert.equal(window.__adminDsPendingAuthFailure, 401, "standalone auth 尚未初始化时应保留失效状态");
  const failures = [];
  delete window.__adminDsPendingAuthFailure;
  window.__adminDsHandleAuthFailure = (status) => failures.push(status);
  await api.fetchGatewayJson("/v1/admin/summary");
  const postResult = await api.fetchGatewayJsonPost("/v1/admin/logs/clear", {});

  assert.deepEqual(failures, [401, 401], "读写请求都应通知共享认证控制器");
  assert.equal(postResult.ok, false, "无 JSON body 时仍应保留 HTTP 失败结果");
  assert.equal(postResult.status, 401);
});

test("admin-ds runtime: fetchGatewayJson 成功时返回解析后的 JSON", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const mockData = { residents: [{ resident_id: "alice", nick: "爱丽丝" }] };
  globalThis.fetch = async () => ({
    ok: true, status: 200,
    json: async () => mockData,
    text: async () => JSON.stringify(mockData),
  });
  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });

  assert.equal(typeof api.fetchGatewayJson, "function", "fetchGatewayJson 应暴露为函数");

  const result = await api.fetchGatewayJson("/v1/residents");
  assert.ok(result !== null, "成功时应返回数据");
  assert.ok(result.residents, "应包含 residents 数组");
  assert.equal(result.residents[0].nick, "爱丽丝");
});

test("admin-ds runtime: markUnavailableButton 正确标记不可用按钮", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();

  assert.equal(typeof api.markUnavailableButton, "function", "markUnavailableButton 应暴露为函数");

  const btn = globalThis.document.createElement("button");
  btn.className = "ds-btn";
  api.markUnavailableButton(btn, "功能尚未接入");

  assert.equal(btn.disabled, true, "按钮应被禁用");
  assert.equal(btn.getAttribute("aria-disabled"), "true", "应有 aria-disabled");
  assert.ok(btn.title.includes("功能尚未接入"), "title 应包含原因");
  assert.equal(btn.dataset.disabledReason, "功能尚未接入", "dataset 应记录原因");
});

test("admin-ds runtime: banResident 发送正确的 POST 请求体", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.prompt = () => "测试禁用理由";

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.banResident("ban-target-user", btn);

  const banCall = fetchCalls.find((call) => call.url.includes("/v1/admin/residents/ban"));
  assert.ok(banCall, "应存在 /ban 请求");
  assert.equal(banCall.method, "POST", "应为 POST 方法");

  const body = JSON.parse(banCall.body);
  assert.equal(body.resident_id, "ban-target-user");
  assert.equal(body.reason, "测试禁用理由");
  assert.equal(body.actor_id, "rsaga");
});

test("admin-ds runtime: banResident prompt 取消不发送 ban 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.prompt = () => null;

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.banResident("ban-target-user", btn);

  assert.equal(fetchCalls.some((call) => call.url.includes("/v1/admin/residents/ban")), false);
});

test("admin-ds runtime: banResident 空 residentId 直接返回", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.prompt = () => "不会使用";

  let called = false;
  globalThis.fetch = async () => {
    called = true;
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  called = false;
  const btn = globalThis.document.createElement("button");
  await api.banResident("", btn);

  assert.equal(called, false, "缺少 residentId 时不应请求 Gateway");
});

test("admin-ds runtime: banResident 无 gateway 时提前返回", serial, async () => {
  setupMinimalGlobals(null);
  globalThis.prompt = () => "不会使用";

  let called = false;
  globalThis.fetch = async () => {
    called = true;
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.banResident("ban-target-user", btn);

  assert.equal(called, false, "无 gateway 时不应请求 Gateway");
});

test("admin-ds runtime: unbanResident 发送正确的 POST 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.unbanResident("unban-target-user", btn);

  const call = fetchCalls.find((item) => item.url.includes("/v1/admin/residents/unban"));
  assert.ok(call, "应存在 /unban 请求");
  assert.equal(call.method, "POST", "应为 POST 方法");
  const body = JSON.parse(call.body);
  assert.equal(body.resident_id, "unban-target-user");
  assert.equal(body.actor_id, "rsaga");
});

test("admin-ds runtime: unbanResident 无 gateway 时提前返回", serial, async () => {
  setupMinimalGlobals(null);

  let called = false;
  globalThis.fetch = async () => {
    called = true;
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.unbanResident("unban-target-user", btn);

  assert.equal(called, false, "无 gateway 时不应请求 Gateway");
});

test("admin-ds runtime: unsanctionResident 发送正确的 POST 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.unsanctionResident("sanction-123", btn);

  const call = fetchCalls.find((item) => item.url.includes("/v1/admin/residents/unsanction"));
  assert.ok(call, "应存在 /unsanction 请求");
  assert.equal(call.method, "POST", "应为 POST 方法");
  const body = JSON.parse(call.body);
  assert.equal(body.sanction_id, "sanction-123");
  assert.equal(body.actor_id, "rsaga");
});

test("admin-ds runtime: unsanctionResident 无 gateway 时提前返回", serial, async () => {
  setupMinimalGlobals(null);

  let called = false;
  globalThis.fetch = async () => {
    called = true;
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.unsanctionResident("sanction-123", btn);

  assert.equal(called, false, "无 gateway 时不应请求 Gateway");
});

test("admin-ds runtime: freezeRoom 发送正确的 POST 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.freezeRoom("room:test:lobby", btn);

  const call = fetchCalls.find((item) => item.url.includes("/v1/admin/rooms/freeze"));
  assert.ok(call, "应存在 /freeze 请求");
  assert.equal(call.method, "POST", "应为 POST 方法");
  assert.equal(JSON.parse(call.body).room_id, "room:test:lobby");
});

test("admin-ds runtime: freezeRoom 空 roomId 直接返回", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  let called = false;
  globalThis.fetch = async () => {
    called = true;
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  called = false;
  const btn = globalThis.document.createElement("button");
  await api.freezeRoom("", btn);

  assert.equal(called, false, "缺少 roomId 时不应请求 Gateway");
});

test("admin-ds runtime: unfreezeRoom 发送正确的 POST 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  const fetchCalls = [];
  globalThis.fetch = async (url, init) => {
    fetchCalls.push({ url: String(url), method: init?.method || "GET", body: init?.body });
    return { ok: true, status: 200, json: async () => ({}), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const btn = globalThis.document.createElement("button");
  await api.unfreezeRoom("room:test:lobby", btn);

  const call = fetchCalls.find((item) => item.url.includes("/v1/admin/rooms/unfreeze"));
  assert.ok(call, "应存在 /unfreeze 请求");
  assert.equal(call.method, "POST", "应为 POST 方法");
  assert.equal(JSON.parse(call.body).room_id, "room:test:lobby");
});

test("admin-ds runtime: fetchGatewayJsonPost 发送 JSON POST 请求", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");

  let capturedInit = null;
  globalThis.fetch = async (_url, init) => {
    capturedInit = init;
    return { ok: true, status: 200, json: async () => ({ ok: true }), text: async () => '{"ok":true}' };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const result = await api.fetchGatewayJsonPost("/v1/admin/residents/ban", {
    resident_id: "user",
    reason: "test",
    actor_id: "rsaga",
  });

  assert.equal(capturedInit.method, "POST");
  assert.equal(capturedInit.headers["Content-Type"], "application/json");
  assert.equal(result.ok, true);
});

test("admin-ds runtime: gateway 请求转发本地 session bearer token", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  window.localStorage.setItem("lobster-session-token", "lbst_admin_test");

  const captured = [];
  globalThis.fetch = async (_url, init) => {
    captured.push(init);
    return { ok: true, status: 200, json: async () => ({ ok: true }), text: async () => "{}" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.fetchGatewayJson("/v1/admin/summary");
  await api.fetchGatewayJsonPost("/v1/admin/logs/clear", {});

  assert.ok(captured.length >= 2);
  for (const init of captured.slice(-2)) {
    assert.equal(init.headers.Authorization, "Bearer lbst_admin_test");
  }
});

test("admin-ds runtime: fetchGatewayJsonPost HTTP 失败时返回 ok:false 且无 error 字段", serial, async () => {
  // 这是「HTTP 失败假成功态」bug 的根因：HTTP 4xx/5xx 时返回 {ok:false,status,data}，
  // 没有 error 字段。若调用方用 `if (r.error) ... else { 成功 }` 判定，HTTP 失败会落入 else。
  // 因此写操作的成功判定必须用 r.ok === true，不能靠 !r.error。
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async (_url, _init) => ({
    ok: false, status: 500,
    json: async () => ({ error: "internal server error" }),
    text: async () => '{"error":"internal server error"}',
  });
  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  const result = await api.fetchGatewayJsonPost("/v1/admin/invites", { actor_id: "rsaga" });

  assert.equal(result.ok, false, "HTTP 500 时 ok 必须为 false");
  assert.equal(result.status, 500);
  assert.equal("error" in result, false, "HTTP 失败返回结构无顶层 error 字段——!r.error 判定会漏，必须用 r.ok");
});

test("admin-ds runtime: Gateway 空/失败时不回退邀请码和审计日志 mock", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async (url) => {
    if (url.includes("/v1/admin/invites")) {
      return { ok: true, status: 200, json: async () => [], text: async () => "[]" };
    }
    if (url.includes("/v1/admin/audit-log")) {
      return { ok: true, status: 200, json: async () => ({ events: [], total: 0 }), text: async () => '{"events":[]}' };
    }
    return { ok: false, status: 500, json: async () => ({}), text: async () => "error" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadInviteCodes();
  await api.loadAuditLog();

  assert.deepEqual(api.getInviteCodes(), [], "Gateway 空邀请码应显示空态，不能保留本地 mock");
  assert.deepEqual(api.getLogs(), [], "Gateway 空审计日志应显示空态，不能保留本地 mock");
  assert.equal(_dsElements.logAlertBadge.textContent, "0", "Gateway 空审计日志时日志 badge 必须为 0");
  assert.equal(_dsElements.logAlertBadge.style.display, "none", "Gateway 空审计日志时日志 badge 必须隐藏");
});

test("admin-ds runtime: Gateway 次级读取失败时清空旧邀请码和审计日志 mock", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async () => ({
    ok: false,
    status: 503,
    json: async () => ({ error: "temporarily unavailable" }),
    text: async () => '{"error":"temporarily unavailable"}',
  });

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadInviteCodes();
  await api.loadAuditLog();

  assert.deepEqual(api.getInviteCodes(), [], "Gateway 邀请码读取失败时不能回退本地 mock");
  assert.deepEqual(api.getLogs(), [], "Gateway 审计日志读取失败时不能回退本地 mock");
});

test("admin-ds runtime: Gateway 主投影空数组不回退居民房间消息 mock", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async (url) => {
    if (url.includes("/v1/admin/residents")) {
      return { ok: true, status: 200, json: async () => [], text: async () => "[]" };
    }
    if (url.includes("/v1/shell/state")) {
      const emptyState = { conversation_shell: { conversations: [] } };
      return { ok: true, status: 200, json: async () => emptyState, text: async () => JSON.stringify(emptyState) };
    }
    if (url.includes("/v1/admin/summary")) {
      const emptySummary = { resident_count: 0, room_count: 0, message_count: 0, online_count: 0, state_version: 1 };
      return { ok: true, status: 200, json: async () => emptySummary, text: async () => JSON.stringify(emptySummary) };
    }
    if (url.includes("/v1/admin/audit-log")) {
      const emptyAudit = { events: [], total: 0 };
      return { ok: true, status: 200, json: async () => emptyAudit, text: async () => JSON.stringify(emptyAudit) };
    }
    if (url.includes("/v1/admin/invites") || url.includes("/v1/admin/permission-groups")) {
      return { ok: true, status: 200, json: async () => [], text: async () => "[]" };
    }
    return { ok: false, status: 404, json: async () => ({}), text: async () => "not found" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadGatewayAdminData();

  assert.deepEqual(api.getResidents(), [], "Gateway 空居民投影应显示空态，不能保留本地 mock");
  assert.deepEqual(api.getRooms(), [], "Gateway 空房间投影应显示空态，不能保留本地 mock");
  assert.deepEqual(api.getMessages(), [], "Gateway 空消息投影应显示空态，不能保留本地 mock");
  assert.equal(_dsElements.statGateway.textContent, "在线", "有效空投影仍应标记 Gateway 在线");
});

test("admin-ds runtime: Gateway 主投影 HTTP 失败清空居民房间消息 mock", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async () => ({
    ok: false,
    status: 503,
    json: async () => ({ error: "temporarily unavailable" }),
    text: async () => '{"error":"temporarily unavailable"}',
  });

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadGatewayAdminData();

  assert.deepEqual(api.getResidents(), [], "Gateway 居民读取失败时不能回退本地 mock");
  assert.deepEqual(api.getRooms(), [], "Gateway 房间读取失败时不能回退本地 mock");
  assert.deepEqual(api.getMessages(), [], "Gateway 消息读取失败时不能回退本地 mock");
  assert.equal(_dsElements.statGateway.textContent, "部分同步", "主投影失败应明确显示部分同步");
  assert.match(_dsElements.statGatewaySub.textContent, /部分数据读取失败/, "主投影失败应说明读取失败");
});

test("admin-ds runtime: Gateway 空权限组不回退内置展示 mock", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  globalThis.fetch = async (url) => {
    if (url.includes("/v1/admin/permission-groups")) {
      return { ok: true, status: 200, json: async () => [], text: async () => "[]" };
    }
    return { ok: false, status: 503, json: async () => ({}), text: async () => "error" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadPermissionGroups();

  assert.deepEqual(api.permissionGroupItems(), [], "Gateway 空权限组应显示空态，不能回退内置 mock");
});

test("admin-ds runtime: 无 Gateway 时权限组保留本地内置说明", serial, async () => {
  setupMinimalGlobals();
  const api = await loadAdminDsWithExports();

  assert.equal(api.permissionGroupItems().length, 4, "离线本地预览仍应保留四类内置权限说明");
});

test("admin-ds runtime: 世界公告失败时清空旧投影并显示空态", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  let failed = false;
  globalThis.fetch = async (url) => {
    if (url.includes("/v1/world-square")) {
      if (failed) return { ok: false, status: 503, json: async () => ({}), text: async () => "error" };
      const payload = [{ posted_at_ms: 1, title: "正式公告", body: "公告正文", severity: "info", author_id: "alice", tags: [] }];
      return { ok: true, status: 200, json: async () => payload, text: async () => JSON.stringify(payload) };
    }
    return { ok: false, status: 503, json: async () => ({}), text: async () => "error" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadWorldNotices();
  assert.equal(api.getWorldNotices().length, 1, "公告成功读取后应有一条正式投影");

  failed = true;
  await api.loadWorldNotices();
  assert.deepEqual(api.getWorldNotices(), [], "公告读取失败时不能保留旧投影");
  const lastNoticeRow = _dsElements.worldNoticeTableBody.children.at(-1);
  assert.match(lastNoticeRow?.children?.[0]?.textContent || "", /暂无世界公告/, "失败后应显示公告空态");
});

test("admin-ds runtime: 安全治理失败时清空通告举报制裁旧投影", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  let failed = false;
  globalThis.fetch = async (url) => {
    if (url.includes("/v1/world-safety")) {
      if (failed) return { ok: false, status: 503, json: async () => ({}), text: async () => "error" };
      const payload = {
        advisories: [{ issued_at_ms: 1, subject_kind: "resident", subject_ref: "alice", action: "warn", reason: "测试", issued_by: "admin" }],
        reports: [{ report_id: "report-1", target_kind: "resident", target_ref: "alice", reporter_id: "bob", summary: "测试举报", status: "Submitted" }],
        resident_sanctions: [{ resident_id: "alice", reason: "测试制裁", status: "active", issued_by: "admin" }],
      };
      return { ok: true, status: 200, json: async () => payload, text: async () => JSON.stringify(payload) };
    }
    return { ok: false, status: 503, json: async () => ({}), text: async () => "error" };
  };

  const api = await loadAdminDsWithExports({ fetchMock: globalThis.fetch });
  await api.loadSafetyData();
  assert.equal(api.getSafetyAdvisories().length, 1, "安全通告成功读取后应有正式投影");
  assert.equal(api.getSafetyReports().length, 1, "安全举报成功读取后应有正式投影");
  assert.equal(api.getResidentSanctions().length, 1, "居民制裁成功读取后应有正式投影");

  failed = true;
  await api.loadSafetyData();
  assert.deepEqual(api.getSafetyAdvisories(), [], "安全通告失败时不能保留旧投影");
  assert.deepEqual(api.getSafetyReports(), [], "安全举报失败时不能保留旧投影");
  assert.deepEqual(api.getResidentSanctions(), [], "居民制裁失败时不能保留旧投影");
});

test("admin-ds runtime: 外部数据写入使用安全 DOM API", serial, async () => {
  const js = await readText("../admin-ds.js");

  // 禁止 innerHTML
  assert.doesNotMatch(js, /\.innerHTML\s*=/, "不应使用 .innerHTML =");
  assert.doesNotMatch(js, /insertAdjacentHTML/, "不应使用 insertAdjacentHTML");
  // 必须使用安全 API
  assert.match(js, /\.textContent\s*=/, "应使用 .textContent =");
  assert.match(js, /document\.createTextNode\(/, "应使用 createTextNode");
  assert.match(js, /document\.createElement\(/, "应使用 createElement");
});

// ====== 批量操作结果汇总（禁止假成功态）======
// ACTIVE-im 规则：写操作失败要有反馈，不能假成功态。
// fetchGatewayJsonPost 永不 reject（内部 try/catch 返回 {error} 或 {ok,data}），
// 因此 Promise.all(...).then 永远触发——批量通过若不逐条检查结果，全部失败也会报"已批量通过"。
// summarizeBatchResults 是纯函数，把 results 数组汇总成 {total, ok, fail}，供回调如实反馈。

test("admin-ds runtime: summarizeBatchResults 暴露为函数", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  assert.equal(typeof api.summarizeBatchResults, "function", "summarizeBatchResults 应暴露为函数");
});

test("admin-ds runtime: summarizeBatchResults 全部成功时 fail=0", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  const s = api.summarizeBatchResults([{ ok: true, data: {} }, { ok: true, data: {} }]);
  assert.equal(s.total, 2);
  assert.equal(s.ok, 2);
  assert.equal(s.fail, 0, "全部成功时 fail 必须为 0");
});

test("admin-ds runtime: summarizeBatchResults 全部失败时 ok=0（禁止假成功态）", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  // 模拟 gateway 全部 401/500：fetchGatewayJsonPost 返回 {error} 或 {ok:false}
  const s = api.summarizeBatchResults([{ error: "401" }, { ok: false, status: 500 }, { error: "network" }]);
  assert.equal(s.total, 3);
  assert.equal(s.ok, 0, "全部失败时 ok 必须为 0，禁止报成功");
  assert.equal(s.fail, 3, "全部失败时 fail 必须等于总数");
});

test("admin-ds runtime: summarizeBatchResults 部分成功如实统计", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  const s = api.summarizeBatchResults([{ ok: true, data: {} }, { error: "401" }, { ok: false, status: 500 }, { ok: true, data: {} }]);
  assert.equal(s.total, 4);
  assert.equal(s.ok, 2);
  assert.equal(s.fail, 2);
});

test("admin-ds runtime: summarizeBatchResults 空数组返回零计数", serial, async () => {
  setupMinimalGlobals("http://127.0.0.1:8787");
  const api = await loadAdminDsWithExports();
  const s = api.summarizeBatchResults([]);
  assert.equal(s.total, 0);
  assert.equal(s.ok, 0);
  assert.equal(s.fail, 0);
});
