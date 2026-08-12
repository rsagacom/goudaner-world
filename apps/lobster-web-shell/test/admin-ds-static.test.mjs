/* ============================================================
   admin-ds-static.test.mjs — admin-ds 正式后台静态安全与结构测试
   不依赖浏览器，不依赖截图，纯文件内容校验。
   ============================================================ */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

async function readShellPage(name) {
  return fs.readFile(new URL(`../${name}`, import.meta.url), "utf8");
}

async function readShellModule(name) {
  return fs.readFile(new URL(`../${name}`, import.meta.url), "utf8");
}

function sliceBetween(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `missing start marker: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

// ====== 文件存在性 ======

test("admin-ds 正式后台文件完整存在", async () => {
  for (const name of ["admin-ds.html", "styles.admin-ds.css", "admin-ds.js", "admin-ds-data.js"]) {
    try {
      await fs.stat(new URL(`../${name}`, import.meta.url));
    } catch {
      assert.fail(`缺少文件: ${name}`);
    }
  }
});

// ====== HTML 引用校验 ======

test("admin-ds.html 引用正确的 CSS 和 JS", async () => {
  const html = await readShellPage("admin-ds.html");

  assert.match(html, /href="\.\/styles\.admin-ds\.css(?:\?v=[^"]+)?"/, "应引用独立样式文件 styles.admin-ds.css");
  assert.match(html, /src="\.\/admin-ds-data\.js"/, "应引用数据文件 admin-ds-data.js");
  assert.match(html, /src="\.\/admin-ds\.js(?:\?v=[^"]+)?"/, "应引用交互脚本 admin-ds.js（允许 ?v= 缓存升版）");
  assert.match(html, /import \{ initStandaloneAuthSurface \} from "\.\/shell-auth-standalone\.js";/, "注册登录应走共享 standalone auth 接线");
  assert.match(html, /initStandaloneAuthSurface\(\{[\s\S]*gatewayUrl/, "应初始化 standalone auth 并传入 gatewayUrl");
  assert.match(html, /onIdentityChanged:\s*\(\) => window\.__adminDsRefresh\?\.\(\)/, "登录状态变化后应刷新后台 Gateway 投影");
  assert.doesNotMatch(html, /import \{ initAuth \} from "\.\/shell-auth\.js";/, "后台页不应复制 shell-auth 初始化细节");
  assert.match(html, /<html lang="zh-CN"/, "应声明中文语言");
  assert.match(html, /<title>AJW聊天 · 管理后台<\/title>/, "应有正确的页面标题");
  assert.match(html, /正式后台 · 单城 IM 运营台/, "应明确作为正式后台而非候选页");
  assert.doesNotMatch(html, /候选方案/, "正式后台页面不应继续标记为候选方案");
});

test("standalone auth module wires OTP submit lifecycle", async () => {
  const js = await readShellModule("shell-auth-standalone.js");

  assert.match(js, /createAuthController\(els,/, "应复用实例化 shell-auth 控制器初始化 DOM refs");
  assert.doesNotMatch(js, /\binitAuth\s*\(/, "独立登录页不应复用模块级认证单例");
  assert.match(js, /loadAuthDraft\(\)/, "初始化后应恢复验证码登录草稿");
  assert.match(js, /updateAuthFormState\(\)/, "初始化和提交后应刷新按钮状态");
  assert.match(js, /requestFormEl\?\.addEventListener\("submit"[\s\S]*requestEmailOtp\(\)/, "申请表单 submit 应调用 requestEmailOtp");
  assert.match(js, /verifyFormEl\?\.addEventListener\("submit"[\s\S]*verifyEmailOtp\(\)/, "校验表单 submit 应调用 verifyEmailOtp");
  assert.match(js, /postGatewayJsonAuthenticated[\s\S]*Authorization:\s*`Bearer \$\{token\}`/, "退出登录应把当前 session token 发送给 Gateway");
  assert.match(js, /authController\.logout\(\)/, "后台 HUD 登录按钮应支持服务端 logout");
  assert.match(js, /persistAuthDraft\(\)/, "submit 前应保存当前邮箱/昵称/验证码草稿");
  assert.match(js, /localizedRuntimeError\(/, "失败反馈应复用本地化错误文案");
});

// ====== 核心模块 DOM 存在 ======

test("admin-ds.html 包含全部核心模块面板", async () => {
  const html = await readShellPage("admin-ds.html");

  // 9个模块容器必须存在
  const modules = [
    { id: "mod-dashboard", label: "仪表盘" },
    { id: "mod-residents", label: "居民管理" },
    { id: "mod-rooms", label: "会话与房间" },
    { id: "mod-messages", label: "消息审核" },
    { id: "mod-permissions", label: "权限与邀请" },
    { id: "mod-world-notices", label: "世界公告" },
    { id: "mod-safety-advisories", label: "安全通告" },
    { id: "mod-sysconfig", label: "系统配置" },
    { id: "mod-devices", label: "设备管理" },
    { id: "mod-scene", label: "场景编辑" },
    { id: "mod-logs", label: "日志与告警" },
  ];

  for (const mod of modules) {
    assert.match(html, new RegExp(`id="${mod.id}"`), `模块 ${mod.label} (${mod.id}) 应存在`);
    assert.match(html, new RegExp(`class="ds-module[^"]*"`), "应有 ds-module class");
  }

  // 侧栏导航项
  const navModules = ["dashboard", "residents", "rooms", "messages", "permissions", "world-notices", "safety-advisories", "sysconfig", "devices", "scene", "logs"];
  for (const nav of navModules) {
    assert.match(html, new RegExp(`data-module="${nav}"`), `导航项 data-module="${nav}" 应存在`);
  }
});

test("admin-ds 房间加入规则没有 Gateway 合同时显示禁用原因", async () => {
  const html = await readShellPage("admin-ds.html");
  const roomRules = sliceBetween(
    html,
    '<span class="ds-card-title">房间加入规则</span>',
    '<!-- 邀请码 -->',
  );

  assert.match(roomRules, /data-state="unavailable"/, "无正式合同的房间规则应明确标记不可用");
  assert.match(roomRules, /Gateway 合同未提供房间加入规则/, "应说明不可用的根因");
  assert.doesNotMatch(roomRules, /#主城大厅|#望海别墅|#世界广场|#AJW文学社/, "不能继续展示硬编码房间规则");
});

test("admin-ds 系统服务健康没有 Gateway 合同时显示禁用原因", async () => {
  const html = await readShellPage("admin-ds.html");
  const serviceHealth = sliceBetween(html, '<div class="ds-config-grid"', '<div class="ds-page-section"');

  assert.match(serviceHealth, /data-state="unavailable"/, "无正式合同的服务健康应明确标记不可用");
  assert.match(serviceHealth, /Gateway 合同未提供服务健康明细/, "应说明服务健康不可用的根因");
  assert.doesNotMatch(serviceHealth, /gw\.lobster-chat\.io|smtp\.lobster-chat\.io|GPT-4o-mini|Claude Sonnet|2\.3 GB|hooks\.lobster-chat\.io/, "不能继续展示硬编码服务健康样例");
});

test("admin-ds 仪表盘没有 Gateway 时不展示硬编码系统事件", async () => {
  const html = await readShellPage("admin-ds.html");
  const events = sliceBetween(html, '<div class="ds-event-list"', '<span class="ds-card-title">Gateway 连接详情</span>');

  assert.match(events, /data-state="unavailable"/, "无 Gateway 的系统事件应明确标记不可用");
  assert.match(events, /Gateway 未连接/, "应说明暂无实时审计事件的根因");
  assert.doesNotMatch(events, /resident_demo|192\.0\.2\.1|gpt-4o-mini|14:32|14:28/, "不能继续展示硬编码系统事件样例");
});

test("admin-ds.html 设备管理模块位于后台主内容区内", async () => {
  const html = await readShellPage("admin-ds.html");
  const mainContent = sliceBetween(html, '<div class="ds-content" id="dsContent">', '</div><!-- /.ds-content -->');

  assert.match(mainContent, /id="mod-devices"/, "设备管理必须位于 #dsContent 内，才能跟随后台主内容布局和模块切换");
});

// ====== 侧栏与布局结构 ======

test("admin-ds.html 包含左侧导航、顶部状态栏、详情面板", async () => {
  const html = await readShellPage("admin-ds.html");

  // 侧栏
  assert.match(html, /<aside class="ds-sidebar"/, "应有侧栏");
  assert.match(html, /id="dsSidebarToggle"/, "应有侧栏切换按钮");
  assert.match(html, /class="ds-sidebar-mobile-overlay"/, "应有移动端遮罩");

  // 顶部状态栏
  assert.match(html, /class="ds-topbar"/, "应有顶部状态栏");
  assert.match(html, /id="dsGatewayStatus"/, "应有 Gateway 状态指示器");
  assert.match(html, /id="dsOnlineCount"/, "应有在线人数显示");
  assert.match(html, /id="dsAlertCount"/, "应有告警数显示");
  assert.match(html, /id="msgAuditBadge"[^>]*aria-label="待处理消息数"/, "消息审核 badge 应有明确的真实数据语义");
  assert.match(html, /id="logAlertBadge"[^>]*aria-label="日志告警数"/, "日志告警 badge 应有明确的真实数据语义");

  // 仪表盘必须能被真实数据刷新，不能只展示硬编码样例
  for (const id of [
    "statGatewaySub",
    "statOnlineResidents",
    "statOnlineSub",
    "statTodayMessages",
    "statMessageSub",
    "statPendingAlerts",
    "statAlertSub",
    "dsGatewayEndpoint",
    "dsGatewayConnection",
    "dsGatewayResident",
    "dsGatewayRoomCount",
    "dsGatewayMessageCount",
    "dsGatewayLastSync",
  ]) {
    assert.match(html, new RegExp(`id="${id}"`), `仪表盘真实数据节点 ${id} 应存在`);
  }

  // 详情面板
  assert.match(html, /id="dsDetailPanel"/, "应有详情面板");
  assert.match(html, /id="dsDetailTitle"/, "应有详情标题");
  assert.match(html, /id="dsDetailBody"/, "应有详情内容区");
  assert.match(html, /id="dsDetailActions"/, "应有详情操作区");
  assert.match(html, /id="dsDetailClose"/, "应有关闭详情按钮");
});

// ====== 表格容器存在 ======

test("admin-ds.html 包含所有数据表格容器", async () => {
  const html = await readShellPage("admin-ds.html");

  const tableBodies = [
    "residentTableBody",
    "roomTableBody",
    "msgTableBody",
    "inviteTableBody",
    "logTableBody",
    "worldNoticeTableBody",
    "safetyAdvisoryTableBody",
    "safetyReportTableBody",
    "sanctionTableBody",
    "deviceTableBody",
  ];

  for (const id of tableBodies) {
    assert.match(html, new RegExp(`id="${id}"`), `表格 tbody ${id} 应存在`);
  }
});

// ====== 搜索与筛选控件 ======

test("admin-ds.html 包含搜索和筛选 UI", async () => {
  const html = await readShellPage("admin-ds.html");

  const controls = [
    "residentSearch", "residentStatusFilter", "residentRoleFilter",
    "roomSearch", "roomTypeFilter",
    "msgSearch", "msgRoomFilter", "msgStatusFilter",
    "logSearch", "logLevelFilter", "logTypeFilter",
    "worldNoticeTitle", "worldNoticeBody", "worldNoticeSeverity", "worldNoticeTags",
    "safetyAdvisorySubjectKind", "safetyAdvisorySubjectRef", "safetyAdvisoryAction", "safetyAdvisoryReason",
    "deviceAddressInput", "deviceLabelInput", "deviceAddBtn",
    "sceneRoomSelect", "sceneEditorContainer",
  ];

  for (const id of controls) {
    assert.match(html, new RegExp(`id="${id}"`), `控件 ${id} 应存在`);
  }
});

// ====== admin-ds.js 安全性：禁止 innerHTML 拼接 ======

test("admin-ds 场景模块提供 scene-editor 可视化编辑器入口", async () => {
  const js = await readShellModule("admin-ds.js");

  // URL 合同与 app.js sceneEditorUrlForCurrentState() 一致：gateway/room/token/identity
  assert.match(js, /'\.\/scene-editor\.html\?gateway=' \+ encodeURIComponent\(gatewayUrl\)/);
  assert.match(js, /'&room=' \+ encodeURIComponent\(room\.id\)/);
  assert.match(js, /'&token=' \+ encodeURIComponent\(editorSessionToken\)/);
  assert.match(js, /'&identity=' \+ encodeURIComponent\(currentGatewayIdentity\(\)\)/);
  assert.match(js, /打开可视化编辑器/);
  // 新标签页打开且不带 opener
  assert.match(js, /target:\s*'_blank'/);
  assert.match(js, /rel:\s*'noopener'/);
});

test("admin-ds.js 不包含 innerHTML 数据拼接", async () => {
  const js = await readShellModule("admin-ds.js");

  // 这些模式表示用 innerHTML 拼接数据对象
  const dangerousPatterns = [
    /detailBody\.innerHTML\s*=/,
    /detailActions\.innerHTML\s*=/,
    /tbody\.innerHTML\s*=/,
  ];

  for (const pattern of dangerousPatterns) {
    assert.doesNotMatch(js, pattern, `不应出现 ${pattern} 模式`);
  }

  // map().join("") 模式（innerHTML 拼接的典型特征）
  // 只有注释中提到，实际代码中不应出现紧邻 innerHTML 的 map-join
  const mapJoinLines = js.split('\n').filter(function (line) {
    return line.includes('.map(') && line.includes('.join(');
  });
  assert.equal(mapJoinLines.length, 0, `不应出现 .map(...).join(...) HTML 拼接模式，实际: ${JSON.stringify(mapJoinLines)}`);

  // 确认使用了安全的 DOM API
  assert.match(js, /document\.createElement\(/, "应使用 document.createElement");
  assert.match(js, /\.textContent\s*=/, "应使用 textContent 写入数据");
  assert.match(js, /document\.createTextNode\(/, "应使用 createTextNode");
});

test("admin-ds.js 读取 gateway 投影并通过统一 helper 执行受控写操作", async () => {
  const js = await readShellModule("admin-ds.js");

  assert.match(js, /function resolveGatewayUrl\(\)/, "应支持 ?gateway= 只读接入");
  assert.match(js, /function loadGatewayAdminData\(\)/, "应有后台只读同步入口");
  assert.match(js, /function updateDashboardSummary\(/, "仪表盘应跟随真实 gateway 或当前投影刷新");
  assert.match(js, /fetchGatewayJson\('\/v1\/admin\/residents'\)/, "应读取居民注册管理投影");
  assert.match(js, /fetchGatewayJson\('\/v1\/shell\/state\?resident_id=' \+ identity\)/, "应读取当前视角 shell state");
  assert.match(js, /normalizeGatewayResidents/, "应把 gateway resident projection 转成后台表格数据");
  assert.match(js, /normalizeGatewayRooms/, "应把 gateway conversations 转成房间表格数据");
  assert.match(js, /normalizeGatewayMessages/, "应把 gateway messages 转成消息审核表格数据");

  assert.match(js, /async function fetchGatewayJsonPost\(/, "真实写操作必须统一走 fetchGatewayJsonPost");
  assert.match(js, /method:\s*'POST'/, "fetchGatewayJsonPost 应使用 JSON POST");
  assert.match(js, /headers:\s*\{\s*'Content-Type':\s*'application\/json'/, "fetchGatewayJsonPost 应设置 JSON Content-Type");

  const directWriteMethods = (js.match(/method:\s*['"](POST|DELETE|PUT|PATCH)['"]/g) || []);
  assert.deepEqual(directWriteMethods, ["method: 'POST'"], "除统一 POST helper 外不应散落直接写方法");

  for (const endpoint of [
    "/v1/admin/messages/moderate",
    "/v1/admin/scene",
    "/v1/admin/rooms/members",
    "/v1/admin/invites/revoke",
    "/v1/admin/logs/handle",
    "/v1/admin/devices/unblock",
    "/v1/admin/devices/block",
    "/v1/admin/devices/remove",
    "/v1/admin/devices/add",
    "/v1/admin/permission-groups",
    "/v1/admin/invites",
    "/v1/admin/residents",
    "/v1/admin/logs/clear",
    "/v1/admin/config",
    "/v1/world-square/notices",
    "/v1/world-safety/reports/review",
    "/v1/world-safety/advisories",
    "/v1/admin/residents/ban",
    "/v1/admin/residents/unban",
    "/v1/admin/residents/unsanction",
    "/v1/admin/rooms/freeze",
    "/v1/admin/rooms/unfreeze",
  ]) {
    assert.match(js, new RegExp(`fetchGatewayJsonPost\\('${endpoint.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}'`), `受控写端点 ${endpoint} 应走统一 helper`);
  }

  assert.doesNotMatch(js, /method:\s*['"]DELETE['"]/, "admin-ds 不应直接开放 DELETE 写接口");
  assert.doesNotMatch(js, /method:\s*['"]PUT['"]/, "admin-ds 不应直接开放 PUT 写接口");
  assert.doesNotMatch(js, /method:\s*['"]PATCH['"]/, "admin-ds 不应直接开放 PATCH 写接口");
  assert.doesNotMatch(js, /Math\.random\(/, "后台在线人数不应再用随机模拟");
});

test("admin-ds 明确区分可用只读动作与待接入写动作", async () => {
  const html = await readShellPage("admin-ds.html");
  const js = await readShellModule("admin-ds.js");

  for (const action of [
    "export-residents",
    "create-resident",
    "batch-approve-messages",
    "refresh-messages",
    "create-permission-group",
    "generate-invite",
    "export-logs",
    "clear-processed-logs",
    "publish-world-notice",
    "publish-safety-advisory",
    "refresh-safety-advisories",
    "refresh-safety-reports",
    "refresh-resident-sanctions",
    "refresh-scene",
  ]) {
    assert.match(html, new RegExp(`data-admin-action="${action}"`), `按钮应声明动作 ${action}`);
  }

  assert.match(js, /function markUnavailableButton\(/, "待接入写动作应统一标记为不可用");
  assert.match(js, /function downloadCsv\(/, "可用导出动作应有真实下载实现");
  assert.match(js, /bindStaticAdminActions\(\)/, "静态按钮应在初始化时统一绑定");
});

// ====== admin-ds-data.js 数据完整性 ======

test("admin-ds-data.js 暴露完整 mock 数据结构", async () => {
  const dataJs = await readShellModule("admin-ds-data.js");

  // 数据数组
  const arrays = ["residents", "rooms", "messages", "inviteCodes", "logs"];
  for (const name of arrays) {
    assert.match(dataJs, new RegExp(`${name}:\\s*\\[`), `应定义 ${name} 数组`);
  }

  // labels 映射表
  assert.match(dataJs, /labels\s*=\s*\{/, "应定义 labels 映射表");
  const labelKeys = [
    "roleTag", "roleText", "statusClass", "statusText",
    "roomTypeTag", "roomTypeText", "msgStatusTag", "msgStatusText",
    "inviteStatusTag", "inviteStatusText", "logTypeText", "logLevelText",
  ];
  for (const key of labelKeys) {
    assert.match(dataJs, new RegExp(`${key}:\\s*\\{`), `labels 应包含 ${key}`);
  }

  // 全局暴露
  assert.match(dataJs, /window\.__ADMIN_DS_DATA__\s*=\s*data/, "应暴露 window.__ADMIN_DS_DATA__");
});

// ====== 旧文件未被修改 ======

test("现有用户端页面未被 admin-ds 修改", async () => {
  const pages = ["creative.html", "index.html", "world-square.html", "admin.html"];

  for (const page of pages) {
    const html = await readShellPage(page);
    // 不应引用 admin-ds 样式
    assert.doesNotMatch(html, /styles\.admin-ds\.css/, `${page} 不应引用 styles.admin-ds.css`);
    // 不应引用 admin-ds 脚本
    assert.doesNotMatch(html, /admin-ds\.js/, `${page} 不应引用 admin-ds.js`);
    assert.doesNotMatch(html, /admin-ds-data\.js/, `${page} 不应引用 admin-ds-data.js`);
  }
});

// ====== CSS 作用域隔离 ======

test("styles.admin-ds.css 使用 .admin-ds 作用域隔离", async () => {
  const css = await readShellModule("styles.admin-ds.css");

  // 主要选择器都应嵌套在 .admin-ds 下
  // 检查是否存在未限定在 .admin-ds 下的全局选择器
  const topLevelSelectors = css
    .split('\n')
    .filter(function (line) {
      var trimmed = line.trim();
      return trimmed &&
             trimmed.indexOf('{') !== -1 &&
             trimmed.indexOf('.admin-ds') === -1 &&
             trimmed.indexOf(':root') === -1 &&
             trimmed.indexOf('/*') !== 0 &&
             trimmed.indexOf('*') !== 0 &&
             trimmed.indexOf('@media') === -1;
    });

  // 允许少量非 admin-ds 开头的选择器（如 @media 内的 .ds-*），但要求绝大多数规则在作用域内
  // 检查文件以 .admin-ds { 开头定义变量
  assert.match(css, /^\.admin-ds\s*\{/m, "CSS 应以 .admin-ds 作用域定义变量开头");

  // 确保没有裸的 body/html 选择器（会污染全局）
  const bareSelectors = css.match(/^(body|html|div|span|table|input|button|a|ul|li|p|h1|h2|h3|h4)\s*[\{,\s]/gm);
  assert.equal(bareSelectors, null, `不应有裸元素选择器污染全局: ${JSON.stringify(bareSelectors)}`);
});

// ====== HTML 不使用内联事件属性 ======

test("admin-ds.html 不包含内联事件处理器", async () => {
  const html = await readShellPage("admin-ds.html");

  assert.doesNotMatch(html, /\son\w+\s*=\s*"/, "不应出现 onclick/onchange 等内联事件属性");
});

// ====== 角色/状态标签值校验 ======

test("admin-ds-data.js 状态与角色映射值合法", async () => {
  const dataJs = await readShellModule("admin-ds-data.js");

  // roleTag 映射
  assert.match(dataJs, /roleTag.*admin.*info/);
  assert.match(dataJs, /roleText.*admin.*管理员/);

  // statusClass 映射
  assert.match(dataJs, /statusClass.*online.*online/);
  assert.match(dataJs, /statusClass.*offline.*offline/);
  assert.match(dataJs, /statusClass.*banned.*banned/);

  // msgStatusTag 包含全部四种状态
  for (const status of ["pending", "passed", "flagged", "blocked"]) {
    assert.match(dataJs, new RegExp(`msgStatusTag.*${status}`), `消息状态 ${status} 应有映射`);
  }
});

// ====== Ban/Unban 管理功能校验 ======

test("admin-ds.js 包含 banResident 和 unbanResident 函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /async function banResident\(/, "应定义 banResident 函数");
  assert.match(js, /async function unbanResident\(/, "应定义 unbanResident 函数");
});

test("admin-ds.js banResident 调用正确的 Gateway 端点", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /\/v1\/admin\/residents\/ban/, "应调用 /v1/admin/residents/ban 端点");
  assert.match(js, /\/v1\/admin\/residents\/unban/, "应调用 /v1/admin/residents/unban 端点");
});

test("admin-ds.js ban/unban 支持按钮加载状态", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /function setBtnLoading\(/, "应定义 setBtnLoading 辅助函数");
  assert.match(js, /function setBtnResult\(/, "应定义 setBtnResult 辅助函数");
  // ban 调用传递 btn 参数
  assert.match(js, /banResident\(resident\.id,\s*banBtn\)/, "表格行 ban 按钮应传递 btn 元素");
  assert.match(js, /unbanResident\(resident\.id,\s*restoreBtn\)/, "表格行 unban 按钮应传递 btn 元素");
  assert.match(js, /banResident\(resident\.id,\s*detailBan\)/, "详情面板 ban 按钮应传递 btn 元素");
  assert.match(js, /unbanResident\(resident\.id,\s*detailRestore\)/, "详情面板 unban 按钮应传递 btn 元素");
  // loading 态设置
  assert.match(js, /btn\.disabled\s*=\s*true/, "loading 时应禁用按钮");
  assert.match(js, /处理中/, "loading 时应显示处理中文本");
  // 结果态设置
  assert.match(js, /ds-btn-success-tick/, "成功时应添加 success-tick 样式");
  assert.match(js, /ds-btn-error-flash/, "失败时应添加 error-flash 样式");
});

test("admin-ds.js normalizeGatewayResidents 处理 is_banned 字段", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /is_banned/, "应读取网关 is_banned 字段");
  assert.match(js, /banned\s*\?\s*'banned'/, "is_banned 为 true 时 status 应为 'banned'");
  // banned 状态优先于 online/offline
  assert.match(js, /status:\s*banned\s*\?\s*'banned'/, "banned 状态应在 online/offline 之前判断");
});

test("admin-ds.js 居民后台读取管理投影而非公开目录", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /fetchGatewayJson\('\/v1\/admin\/residents'\)/, "居民后台应读取含注册审计字段的管理端点");
  assert.doesNotMatch(js, /fetchGatewayJson\('\/v1\/residents'\)/, "居民后台不应继续读取缺少注册账号的公开目录");
});

test("admin-ds.js 包含 freezeRoom 和 unfreezeRoom 函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /async function freezeRoom\(/, "应定义 freezeRoom 函数");
  assert.match(js, /async function unfreezeRoom\(/, "应定义 unfreezeRoom 函数");
});

test("admin-ds.js freezeRoom/unfreezeRoom 调用正确的 Gateway 端点", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /\/v1\/admin\/rooms\/freeze/, "应调用 /v1/admin/rooms/freeze 端点");
  assert.match(js, /\/v1\/admin\/rooms\/unfreeze/, "应调用 /v1/admin/rooms/unfreeze 端点");
});

test("admin-ds.js freeze/unfreeze 支持按钮加载状态", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /freezeRoom\(room\.id,\s*freezeBtn\)/, "房间行 freeze 按钮应传递 btn 元素");
  assert.match(js, /unfreezeRoom\(room\.id,\s*unfreezeBtn\)/, "房间行 unfreeze 按钮应传递 btn 元素");
  assert.match(js, /setBtnLoading\(btn,\s*true\)/, "调用时应进入 loading 状态");
});

// ====== Debug 开关 ======

test("admin-ds.js 启动日志默认关闭，仅 ?debug=1 或 ?debug=true 时输出", async () => {
  const js = await readShellModule("admin-ds.js");

  // debugEnabled 变量存在且默认关闭
  assert.match(js, /var debugEnabled\s*=\s*_debugParam\s*===\s*'1'\s*\|\|\s*_debugParam\s*===\s*'true'/, "应定义 debugEnabled 开关变量");

  // console.log 启动日志必须被 if (debugEnabled) 包裹
  const logLines = js.split("\n");
  let inDebugBlock = false;
  let unconditionalStartupLogs = [];

  for (let i = 0; i < logLines.length; i++) {
    const line = logLines[i];
    if (line.includes("if (debugEnabled)")) {
      inDebugBlock = true;
      continue;
    }
    if (inDebugBlock && line.trim() === "}") {
      inDebugBlock = false;
      continue;
    }
    // 检查是否有在 debugEnabled block 外的 console.log('AJW聊天 或 模块: 或 快捷键:
    if (!inDebugBlock && (line.includes("console.log('AJW聊天") || line.includes("console.log('模块:") || line.includes("console.log('快捷键:"))) {
      unconditionalStartupLogs.push(`line ${i + 1}: ${line.trim()}`);
    }
  }

  assert.equal(unconditionalStartupLogs.length, 0,
    `不应有无条件的启动 console.log: ${JSON.stringify(unconditionalStartupLogs)}`);
});

// ====== 加载/空/错误状态校验 ======

test("admin-ds.js 包含 renderEmptyRow 空状态辅助函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /function renderEmptyRow/, "应定义 renderEmptyRow 函数");
  assert.match(js, /textContent\s*=\s*message/, "renderEmptyRow 应使用 textContent 写入文案");
  assert.match(js, /colspan/, "空状态行应设置 colspan");
});

test("admin-ds.js 包含 setSectionLoading 加载状态辅助函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /function setSectionLoading/, "应定义 setSectionLoading 函数");
  assert.match(js, /dataset\.loading/, "应设置 data-loading 属性");
  assert.match(js, /style\.opacity/, "应设置 opacity 样式");
});

test("admin-ds.js 各渲染函数包含空状态分支", async () => {
  const js = await readShellModule("admin-ds.js");

  // 居民表格空状态
  assert.match(js, /暂无居民数据/, "renderResidents 应有居民空状态");
  assert.match(js, /没有匹配的居民/, "renderResidents 应有搜索空状态");

  // 会话表格空状态
  assert.match(js, /暂无会话数据/, "renderRooms 应有会话空状态");
  assert.match(js, /没有匹配的会话/, "renderRooms 应有搜索空状态");

  // 消息表格空状态
  assert.match(js, /暂无消息数据/, "renderMessages 应有消息空状态");

  // 邀请码表格空状态
  assert.match(js, /暂无邀请码数据/, "renderInvites 应有邀请码空状态");

  // 日志表格空状态
  assert.match(js, /暂无日志数据/, "renderLogs 应有日志空状态");

    // 世界公告空状态
    assert.match(js, /暂无世界公告/, "renderWorldNotices 应有世界公告空状态");

    // 安全通告空状态
    assert.match(js, /暂无安全通告/, "renderSafetyAdvisories 应有安全通告空状态");

    // 安全举报空状态
    assert.match(js, /暂无安全举报/, "renderSafetyReports 应有安全举报空状态");

    // 居民制裁空状态
    assert.match(js, /暂无居民制裁/, "renderResidentSanctions 应有居民制裁空状态");
});

test("admin-ds.js 告警 badge 与仪表盘计数复用同一真实数据投影", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /function updateAlertCounts\(\)/, "应有统一告警计数投影");
  assert.match(js, /logAlertBadge\.textContent = String\(warningLogs\)/, "日志 badge 必须来自 warning/error 日志计数");
  assert.match(js, /renderLogs\(filterLevel, filterType, searchTerm\) \{\s*updateAlertCounts\(\);/, "审计日志异步刷新后必须更新 badge");
  assert.match(js, /updateAlertCounts\(\);\s*\n\s*\}/, "仪表盘刷新应同步告警计数");
});

test("admin-ds.js loadGatewayAdminData 包含加载和错误状态处理", async () => {
  const js = await readShellModule("admin-ds.js");

  // Gateway 未连接
  assert.match(js, /Gateway 未连接/, "应有 Gateway 未连接状态");

  // 加载中
  assert.match(js, /Gateway 同步中/, "应有同步中状态");

  // 成功
  assert.match(js, /Gateway 在线/, "应有在线成功状态");

  // 错误
  assert.match(js, /Gateway 读取失败/, "应有读取失败状态");
  assert.match(js, /部分读取失败/, "应有部分失败状态");

  // Loading 状态管理
  assert.match(js, /setSectionLoading\('mod-residents', true\)/, "应设置居民模块加载态");
  assert.match(js, /setSectionLoading\('mod-residents', false\)/, "应清除居民模块加载态");
});

test("admin-ds.js loadGatewayAdminData 使用 Promise.allSettled 容错", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /Promise\.allSettled/, "应使用 allSettled 而非 all");
  assert.match(js, /status\s*===\s*'fulfilled'/, "应检查每个 promise 状态");
  assert.match(js, /status\s*===\s*'rejected'/, "应处理 rejected promise");
});

// ====== 城邦治理模块 ======

test("admin-ds.js 包含世界公告相关函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /async function loadWorldNotices/, "应定义 loadWorldNotices 函数");
  assert.match(js, /function renderWorldNotices/, "应定义 renderWorldNotices 函数");
  assert.match(js, /async function publishWorldNotice/, "应定义 publishWorldNotice 函数");
  assert.match(js, /\/v1\/world-square/, "应调用 world-square 端点");
  assert.match(js, /\/v1\/world-square\/notices/, "应调用 world-square/notices 端点");
});

test("admin-ds.js 包含安全通告相关函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /async function loadSafetyData/, "应定义 loadSafetyData 函数");
  assert.match(js, /function renderSafetyAdvisories/, "应定义 renderSafetyAdvisories 函数");
  assert.match(js, /function renderSafetyReports/, "应定义 renderSafetyReports 函数");
  assert.match(js, /function renderResidentSanctions/, "应定义 renderResidentSanctions 函数");
  assert.match(js, /async function reviewSafetyReport/, "应定义 reviewSafetyReport 函数");
  assert.match(js, /async function unsanctionResident/, "应定义 unsanctionResident 函数");
  assert.match(js, /async function publishSafetyAdvisory/, "应定义 publishSafetyAdvisory 函数");
  assert.match(js, /\/v1\/world-safety/, "应调用 world-safety 端点");
  assert.match(js, /\/v1\/world-safety\/advisories/, "应调用 world-safety/advisories 端点");
  assert.match(js, /\/v1\/world-safety\/reports\/review/, "应调用 world-safety/reports/review 端点");
  assert.match(js, /\/v1\/admin\/residents\/unsanction/, "应调用 admin/residents/unsanction 端点");
});

test("admin-ds.js 包含场景编辑相关函数", async () => {
  const js = await readShellModule("admin-ds.js");
  assert.match(js, /function loadSceneModule/, "应定义 loadSceneModule 函数");
  assert.match(js, /function renderSceneEditor/, "应定义 renderSceneEditor 函数");
  assert.match(js, /\/v1\/admin\/scene/, "应调用 admin/scene 端点");
});

test("admin-ds scene editor posts paired day/night image URLs", async () => {
  const js = await readShellModule("admin-ds.js");
  const sceneEditor = sliceBetween(js, "function renderSceneEditor(room, container)", "  // ====== Device Management ======");

  assert.match(sceneEditor, /placeholder:\s*'白天背景图 URL（可选）'/, "正式场景编辑器应显示 day URL 输入");
  assert.match(sceneEditor, /placeholder:\s*'夜晚背景图 URL（可选）'/, "正式场景编辑器应显示 night URL 输入");
  assert.match(sceneEditor, /var dayUrl = dayUrlInput\.value\.trim\(\);/, "保存时应读取 day URL");
  assert.match(sceneEditor, /var nightUrl = nightUrlInput\.value\.trim\(\);/, "保存时应读取 night URL");
  // 2026-08-02 去重：载荷构造收口到共享助手，两处调用点共用同一实现
  assert.match(sceneEditor, /var ilPayload = buildImageLayerPayload\(selectedPreset, dayUrl, nightUrl, 'admin-scene-'\);/, "图像层载荷应走共享助手");
  assert.match(sceneEditor, /var hlPayload = buildHotspotLayerPayload\(hotspotEditor\.collectHotspots\(\)\);/, "热点层载荷应走共享助手");
  assert.match(sceneEditor, /image_layer: ilPayload/, "场景保存应透传图像层清除语义");
  assert.match(sceneEditor, /hotspot_layer: hlPayload/, "场景保存应透传热点层清除语义");

  // 共享助手本体：昼夜成对、空则显式 null（清除语义）
  const helper = sliceBetween(js, "function buildImageLayerPayload", "  function ensureAdminNotice");
  assert.match(helper, /if \(!selectedPreset && !dayUrl && !nightUrl\) return null;/, "清除预设和昼夜图片时应显式返回 null");
  assert.match(helper, /day_image_url: dayUrl \|\| null/, "image_layer payload 应包含 day_image_url");
  assert.match(helper, /night_image_url: nightUrl \|\| null/, "image_layer payload 应包含 night_image_url");
  const hlHelper = sliceBetween(js, "function buildHotspotLayerPayload", "  function buildImageLayerPayload");
  assert.match(hlHelper, /if \(!hotspotsOut \|\| !hotspotsOut\.length\) return null;/, "删除全部热点时应显式返回 null");
});

test("admin-ds scene editor keeps hotspot title count aligned with the editable list", async () => {
  const js = await readShellModule("admin-ds.js");
  const sceneEditor = sliceBetween(js, "function renderSceneEditor(room, container)", "  // ====== Device Management ======");

  assert.match(sceneEditor, /var hotspotTitle = el\('span', \{ class: 'ds-card-title' \}/, "热点标题应保留可更新的 DOM 节点");
  assert.match(sceneEditor, /hotspotTitle\.textContent = '热点配置 \(' \+ existingHotspots\.length \+ ' 个\)'/, "重绘热点列表时应同步标题计数");
  assert.match(sceneEditor, /addEventListener\('click', hotspotEditor\.addHotspot\)/, "添加热点应走共享编辑器入口");
  // 共享编辑器内部：添加/删除都必须重绘，且重绘后触发 onRowsRendered 计数回调
  assert.match(js, /existingHotspots\.push\(newHotspotDefaults\(\)\);\s*\n\s*renderRows\(\);/, "添加热点后应触发列表重绘");
  assert.match(js, /existingHotspots\.splice\(idx, 1\);\s*\n\s*renderRows\(\);/, "删除热点后应触发列表重绘");
  assert.match(js, /if \(onRowsRendered\) onRowsRendered\(\);/, "共享编辑器重绘后必须触发计数回调");
});

// ====== 写操作失败反馈（禁止假成功态）======
// ACTIVE-im：fetchGatewayJsonPost 永不 reject，.then 必须检查 r.error/r.ok，
// 否则失败时静默刷新或误报成功。设备 block/unblock/remove 历史上无参 .then 静默 loadDevices。

test("admin-ds.js 设备 block/unblock/remove 操作必须检查失败（禁止静默）", async () => {
  const js = await readShellModule("admin-ds.js");
  for (const path of ["/v1/admin/devices/unblock", "/v1/admin/devices/block", "/v1/admin/devices/remove"]) {
    const escaped = path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const idx = js.indexOf(path);
    assert.ok(idx !== -1, `应调用 ${path} 端点`);
    // 截取该端点调用后 180 字符内的 .then 回调体，必须出现 error 检查
    const snippet = js.slice(idx, idx + 180);
    assert.match(snippet, /error/, `${path} 的 .then 回调必须检查 r.error 并反馈失败，不能静默 loadDevices`);
  }
});

test("admin-ds.js 批量通过消息必须逐条检查结果（禁止假成功态）", async () => {
  const js = await readShellModule("admin-ds.js");
  // 批量通过用 summarizeBatchResults 汇总，回调按 ok/fail 如实反馈
  assert.match(js, /function summarizeBatchResults/, "应定义 summarizeBatchResults 纯函数汇总批量结果");
  const idx = js.indexOf("batch-approve-messages");
  assert.ok(idx !== -1, "应存在批量通过按钮");
  // 批量通过回调体（按钮 wiring 后 1000 字符）必须引用 summarizeBatchResults，不能无条件 success
  const snippet = js.slice(idx, idx + 1000);
  assert.match(snippet, /summarizeBatchResults/, "批量通过回调必须用 summarizeBatchResults 汇总，不能无条件报成功");
});

// ====== HTTP 失败假成功态防护 ======
// fetchGatewayJsonPost 在 HTTP 4xx/5xx 时返回 {ok:false, status, data}，**无 error 字段**。
// 若写操作用 `if (r.error) ... else { 成功 }` 判定，HTTP 失败会落入 else 成功分支，
// 读 r.data.xxx 得到 undefined 却报"已生成/已清空"——假成功态。
// 成功判定必须用 r.ok === true（对齐 await 模式 banResident 的 else if (result.ok)）。

test("admin-ds.js 生成邀请码成功判定用 r.ok 而非仅 !r.error（防 HTTP 失败假成功）", async () => {
  const js = await readShellModule("admin-ds.js");
  const idx = js.indexOf("/v1/admin/invites', {actor_id: currentGatewayIdentity(), max_uses: 10}");
  assert.ok(idx !== -1, "应存在生成邀请码写操作");
  const snippet = js.slice(idx, idx + 400);
  // 成功分支必须显式检查 r.ok，不能是裸 else（裸 else 会把 HTTP 失败当成功）
  assert.match(snippet, /else if \(r\.ok\)|else if \(r && r\.ok\)/, "生成邀请码成功判定必须用 r.ok，禁止裸 else 导致 HTTP 失败假成功");
});

test("admin-ds.js 清空日志成功判定用 r.ok 而非仅 !r.error（防 HTTP 失败假成功）", async () => {
  const js = await readShellModule("admin-ds.js");
  const idx = js.indexOf("/v1/admin/logs/clear', {}");
  assert.ok(idx !== -1, "应存在清空日志写操作");
  const snippet = js.slice(idx, idx + 400);
  assert.match(snippet, /else if \(r\.ok\)|else if \(r && r\.ok\)/, "清空日志成功判定必须用 r.ok，禁止裸 else 导致 HTTP 失败假成功");
});

test("admin-ds.js 所有 .then 写操作成功判定用 r.ok（系统性防 HTTP 失败假成功）", async () => {
  // fetchGatewayJsonPost HTTP 失败返回 {ok:false,status,data} 无 error。
  // 任何 `if (r.error) ... else { 成功 }` 裸 else 都会在 HTTP 失败时假成功。
  // 逐一覆盖所有 .then 模式写操作端点。
  const js = await readShellModule("admin-ds.js");
  const endpoints = [
    "/v1/admin/rooms/members', {room_id:",
    "/v1/admin/invites/revoke', {code:",
    "/v1/admin/logs/handle', {log_id:",
    "/v1/admin/residents', {resident_id: residentId, email: email}",
    "/v1/admin/devices/unblock', { address:",
    "/v1/admin/devices/block', { address:",
    "/v1/admin/devices/remove', { address:",
  ];
  for (const ep of endpoints) {
    const idx = js.indexOf(ep);
    assert.ok(idx !== -1, `应存在写操作端点 ${ep}`);
    const snippet = js.slice(idx, idx + 500);
    assert.match(
      snippet,
      /else if \(r\.ok\)|else if \(r && r\.ok\)/,
      `${ep} 成功判定必须用 r.ok，禁止裸 else 导致 HTTP 失败假成功`
    );
  }
});

test("admin-ds.js await 写操作不能把 HTTP 失败当成功", async () => {
  const js = await readShellModule("admin-ds.js");
  const cases = [
    ["fetchGatewayJsonPost('/v1/admin/devices/add'", "设备添加"],
    ["fetchGatewayJsonPost('/v1/admin/permission-groups'", "权限组创建"],
  ];
  for (const [endpoint, label] of cases) {
    const idx = js.indexOf(endpoint);
    assert.ok(idx !== -1, `应存在 ${label}端点 ${endpoint}`);
    const snippet = js.slice(idx, idx + 620);
    assert.match(
      snippet,
      /else if \((?:res|resp)\.ok\)/,
      `${label}成功判定必须显式使用 res/resp.ok，禁止仅检查 error`
    );
  }
});
