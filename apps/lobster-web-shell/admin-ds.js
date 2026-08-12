/* ============================================================
   admin-ds.js — AJW聊天 正式管理后台交互脚本
   安全规则：所有数据通过 textContent 写入，不使用 innerHTML 拼接。
   Mock 数据来自 admin-ds-data.js（window.__ADMIN_DS_DATA__）。
   ============================================================ */

(function () {
  'use strict';

  var _debugParam = (new URLSearchParams(window.location.search)).get('debug');
  var debugEnabled = _debugParam === '1' || _debugParam === 'true';

  // ====== DOM refs ======
  var sidebar = document.getElementById('dsSidebar');
  var sidebarToggle = document.getElementById('dsSidebarToggle');
  var sidebarOverlay = document.getElementById('dsSidebarOverlay');
  var detailPanel = document.getElementById('dsDetailPanel');
  var detailTitle = document.getElementById('dsDetailTitle');
  var detailBody = document.getElementById('dsDetailBody');
  var detailActions = document.getElementById('dsDetailActions');
  var detailClose = document.getElementById('dsDetailClose');
  var dashboardTime = document.getElementById('dashboardTime');
  var msgAuditBadge = document.getElementById('msgAuditBadge');
  var logAlertBadge = document.getElementById('logAlertBadge');
  var statGateway = document.getElementById('statGateway');
  var statGatewaySub = document.getElementById('statGatewaySub');
  var statOnlineResidents = document.getElementById('statOnlineResidents');
  var statOnlineSub = document.getElementById('statOnlineSub');
  var statTodayMessages = document.getElementById('statTodayMessages');
  var statMessageSub = document.getElementById('statMessageSub');
  var statPendingAlerts = document.getElementById('statPendingAlerts');
  var statAlertSub = document.getElementById('statAlertSub');
  var topbarOnlineCount = document.getElementById('dsOnlineCount');
  var topbarAlertCount = document.getElementById('dsAlertCount');
  var gatewayEndpoint = document.getElementById('dsGatewayEndpoint');
  var gatewayConnection = document.getElementById('dsGatewayConnection');
  var gatewayResident = document.getElementById('dsGatewayResident');
  var gatewayRoomCount = document.getElementById('dsGatewayRoomCount');
  var gatewayMessageCount = document.getElementById('dsGatewayMessageCount');
  var gatewayLastSync = document.getElementById('dsGatewayLastSync');
  var gatewayUptime = document.getElementById('dsGatewayUptime');
  var gatewayStateVersion = document.getElementById('dsGatewayStateVersion');

  // ====== Data ======
  var DS = window.__ADMIN_DS_DATA__;
  var residents = DS.residents;
  var rooms = DS.rooms;
  var messages = DS.messages;
  var inviteCodes = DS.inviteCodes;
  var permissionGroups = [];
  var logs = DS.logs;
  var auditEvents = [];
  var gatewayAuditLogs = [];
  var worldNotices = [];
  var safetyAdvisories = [];
  var safetyReports = [];
  var residentSanctions = [];
  var L = DS.labels;
  // Extend labels for audit event types
  L.logTypeText.audit_security = '安全操作';
  L.logTypeText.audit_content = '内容审核';
  L.logTypeText.audit_permission = '权限变更';
  L.logTypeText.audit_config = '系统配置';
  L.logLevelText.warn = '警告';
  L.logLevelText.info = '信息';
  var gatewayUrl = resolveGatewayUrl();
  var gatewayStatus = document.getElementById('dsGatewayStatus');

  var activeModule = 'dashboard';
  var sidebarExpanded = true;

  // ====== DOM Helpers ======

  /* 创建元素：el('div', {class:'foo', data:{bar:'1'}, style:'color:red'}, child1, child2, ...)
     - children 可以是字符串（自动创建 textNode）或 DOM 节点
     - 不支持内联事件属性，事件通过 addEventListener 绑定 */
  function el(tag, attrs) {
    var element = document.createElement(tag);
    if (attrs) {
      var keys = Object.keys(attrs);
      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        var val = attrs[key];
        if (key === 'class') { element.className = val; }
        else if (key === 'data') {
          var dk = Object.keys(val);
          for (var j = 0; j < dk.length; j++) { element.dataset[dk[j]] = val[dk[j]]; }
        }
        else if (key === 'style' && typeof val === 'string') { element.style.cssText = val; }
        else { element.setAttribute(key, val); }
      }
    }
    for (var k = 2; k < arguments.length; k++) {
      var child = arguments[k];
      if (child == null) continue;
      if (typeof child === 'string') { element.appendChild(document.createTextNode(child)); }
      else { element.appendChild(child); }
    }
    return element;
  }

  function clear(el) { while (el.firstChild) el.removeChild(el.firstChild); }

  function safeLocalStorageGet(key) {
    try { return window.localStorage ? window.localStorage.getItem(key) : null; }
    catch (_) { return null; }
  }

  function safeLocalStorageSet(key, value) {
    try { if (window.localStorage) window.localStorage.setItem(key, value); }
    catch (_) { /* ignore storage failures */ }
  }

  function resolveGatewayUrl() {
    var params = new URLSearchParams(window.location.search);
    var query = params.get('gateway');
    if (query && query.trim()) {
      var normalized = query.trim().replace(/\/+$/, '');
      safeLocalStorageSet('lobster-gateway-url', normalized);
      return normalized;
    }
    var remembered = safeLocalStorageGet('lobster-gateway-url');
    return remembered ? remembered.replace(/\/+$/, '') : null;
  }

  function currentGatewayIdentity() {
    var params = new URLSearchParams(window.location.search);
    return (params.get('identity') || safeLocalStorageGet('lobster-identity') || 'rsaga').trim() || 'rsaga';
  }

  async function fetchGatewayJson(path) {
    if (!gatewayUrl) return null;
    var sessionToken = safeLocalStorageGet('lobster-session-token');
    var response = await fetch(gatewayUrl + path, {
      headers: { Accept: 'application/json', ...(sessionToken ? { Authorization: 'Bearer ' + sessionToken } : {}) }
    });
    if (!response.ok) return null;
    return response.json();
  }

  async function fetchGatewayJsonPost(path, body) {
    if (!gatewayUrl) return { error: 'Gateway 未连接' };
    try {
      var sessionToken = safeLocalStorageGet('lobster-session-token');
      var response = await fetch(gatewayUrl + path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json', ...(sessionToken ? { Authorization: 'Bearer ' + sessionToken } : {}) },
        body: JSON.stringify(body)
      });
      var data = await response.json();
      return { ok: response.ok, status: response.status, data: data };
    } catch (err) {
      return { error: err.message || '请求失败' };
    }
  }

  async function moderateMessage(messageId, conversationId, action) {
    if (!messageId || !conversationId) {
      return { error: '缺少 message_id 或 conversation_id' };
    }
    return fetchGatewayJsonPost('/v1/admin/messages/moderate', {
      message_id: messageId,
      conversation_id: conversationId,
      action: action
    });
  }

  // 汇总批量写操作结果。fetchGatewayJsonPost 永不 reject（内部 try/catch 返回
  // {error} 或 {ok,data}），因此 Promise.all(...).then 永远触发——调用方必须用本函数
  // 逐条判定成功/失败并如实反馈，禁止无条件报成功（ACTIVE-im：不能假成功态）。
  function summarizeBatchResults(results) {
    var list = Array.isArray(results) ? results : [];
    var ok = 0;
    for (var i = 0; i < list.length; i++) {
      var r = list[i];
      if (r && !r.error && r.ok !== false) ok++;
    }
    return { total: list.length, ok: ok, fail: list.length - ok };
  }

  function setGatewayStatus(text, className) {
    if (!gatewayStatus) return;
    gatewayStatus.textContent = text;
    gatewayStatus.className = 'ds-status-dot ' + className;
  }

  function refreshCurrentMessageView() {
    renderMessages(
      document.getElementById('msgRoomFilter').value,
      document.getElementById('msgStatusFilter').value,
      document.getElementById('msgSearch').value
    );
  }

  function formatNumber(value) {
    return Number(value || 0).toLocaleString('zh-CN');
  }

  function renderEmptyRow(tbody, colspan, message) {
    clear(tbody);
    var tr = el('tr');
    var td = el('td', { attrs: { colspan: String(colspan) }, style: 'text-align:center;padding:2rem;color:var(--ds-text-secondary);' });
    td.textContent = message;
    tr.appendChild(td);
    tbody.appendChild(tr);
  }

  function setSectionLoading(sectionId, isLoading) {
    var el = document.getElementById(sectionId);
    if (!el) return;
    if (isLoading) {
      el.dataset.loading = 'true';
      el.style.opacity = '0.6';
    } else {
      delete el.dataset.loading;
      el.style.opacity = '';
    }
  }

  // ---- 前端分页 ----
  var PAGE_SIZE = 25;
  var pageState = { residents: 1, rooms: 1, messages: 1, logs: 1, permissions: 1 };

  function paginateArray(arr, page) {
    var start = (page - 1) * PAGE_SIZE;
    return arr.slice(start, start + PAGE_SIZE);
  }

  function renderPagination(moduleName, totalItems, onPageChange) {
    var currentPage = pageState[moduleName] || 1;
    var totalPages = Math.max(1, Math.ceil(totalItems / PAGE_SIZE));
    if (currentPage > totalPages) { currentPage = totalPages; pageState[moduleName] = currentPage; }
    var module = document.getElementById('mod-' + moduleName);
    if (!module) return;
    var paginationEl = module.querySelector('.ds-pagination');
    if (!paginationEl) return;

    // 更新信息行
    var infoEl = paginationEl.querySelector('.ds-pagination-info');
    if (infoEl) {
      clear(infoEl);
      infoEl.appendChild(document.createTextNode('共 '));
      var strongCount = el('strong');
      strongCount.textContent = String(totalItems);
      infoEl.appendChild(strongCount);
      infoEl.appendChild(document.createTextNode(' 条' + (totalPages > 1 ? '，第 ' + currentPage + '/' + totalPages + ' 页' : '') + '（前端分页）'));
    }

    // 更新按钮
    var btnsEl = paginationEl.querySelector('.ds-pagination-btns');
    if (!btnsEl) return;
    clear(btnsEl);

    var addPageBtn = function (label, targetPage, isDisabled, isActive) {
      var btn = el('button', { class: 'ds-page-btn' + (isActive ? ' active' : '') });
      btn.textContent = String(label);
      if (isDisabled) btn.disabled = true;
      else btn.addEventListener('click', function () { pageState[moduleName] = targetPage; onPageChange(targetPage); });
      btnsEl.appendChild(btn);
    };

    addPageBtn('‹', currentPage - 1, currentPage <= 1, false);
    for (var p = 1; p <= totalPages; p++) {
      addPageBtn(p, p, false, p === currentPage);
    }
    addPageBtn('›', currentPage + 1, currentPage >= totalPages, false);
  }

  function countPendingMessages() {
    var count = 0;
    for (var i = 0; i < messages.length; i++) {
      if (messages[i].status === 'pending' || messages[i].status === 'flagged' || messages[i].status === 'blocked') count++;
    }
    return count;
  }

  function countWarningLogs() {
    var count = 0;
    for (var i = 0; i < logs.length; i++) {
      if (logs[i].level === 'error' || logs[i].level === 'warn') count++;
    }
    return count;
  }

  function updateAlertCounts() {
    var pendingMessages = countPendingMessages();
    var warningLogs = countWarningLogs();
    var alertTotal = pendingMessages + warningLogs;
    if (msgAuditBadge) {
      msgAuditBadge.textContent = String(pendingMessages);
      msgAuditBadge.style.display = pendingMessages > 0 ? '' : 'none';
    }
    if (logAlertBadge) {
      logAlertBadge.textContent = String(warningLogs);
      logAlertBadge.style.display = warningLogs > 0 ? '' : 'none';
    }
    if (statPendingAlerts) statPendingAlerts.textContent = formatNumber(alertTotal);
    if (statAlertSub) statAlertSub.textContent = '消息审核 ' + formatNumber(pendingMessages) + ' · 日志告警 ' + formatNumber(warningLogs);
    if (topbarAlertCount) topbarAlertCount.textContent = '告警 ' + formatNumber(alertTotal);
  }

  function updateGatewayConnectionTag(text, tagClass) {
    if (!gatewayConnection) return;
    gatewayConnection.textContent = text;
    gatewayConnection.className = 'ds-tag ' + tagClass;
  }

  function updateDashboardSummary(source, summary) {
    var hasGateway = source === 'gateway' || source === 'gateway-partial';
    var partialGateway = source === 'gateway-partial';
    var currentIdentity = currentGatewayIdentity();
    var syncLabel = new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });

    var residentCount, roomCount, messageCount, onlineCount, uptimeSec;
    if (hasGateway && !partialGateway && summary && typeof summary.resident_count === 'number') {
      residentCount = summary.resident_count;
      roomCount = summary.room_count;
      messageCount = summary.message_count;
      onlineCount = summary.online_count;
      uptimeSec = summary.gateway_uptime_ms ? Math.floor(summary.gateway_uptime_ms / 1000) : 0;
    } else {
      residentCount = residents.length;
      roomCount = rooms.length;
      messageCount = messages.length;
      onlineCount = residents.filter(function (r) { return r.status === 'online'; }).length;
      uptimeSec = 0;
    }

    if (statGateway) statGateway.textContent = partialGateway ? '部分同步' : (hasGateway ? '在线' : '本地');
    if (statGatewaySub) {
      if (partialGateway) {
        statGatewaySub.textContent = gatewayUrl + ' · 部分数据读取失败 · 当前居民 ' + currentIdentity;
      } else if (hasGateway) {
        statGatewaySub.textContent = gatewayUrl + ' · 当前居民 ' + currentIdentity + (uptimeSec ? ' · 运行 ' + Math.floor(uptimeSec / 3600) + 'h ' + Math.floor((uptimeSec % 3600) / 60) + 'm' : '');
      } else {
        statGatewaySub.textContent = '本地预览数据 · 未连接 Gateway';
      }
    }
    if (statOnlineResidents) statOnlineResidents.textContent = formatNumber(onlineCount);
    if (statOnlineSub) statOnlineSub.textContent = '居民总数 ' + formatNumber(residentCount);
    if (statTodayMessages) statTodayMessages.textContent = formatNumber(messageCount);
    if (statMessageSub) statMessageSub.textContent = '可见会话 ' + formatNumber(roomCount);
    if (topbarOnlineCount) topbarOnlineCount.textContent = '在线 ' + formatNumber(onlineCount) + ' 人';
    if (gatewayEndpoint) gatewayEndpoint.textContent = gatewayUrl || '未连接';
    if (gatewayResident) gatewayResident.textContent = currentIdentity;
    if (gatewayRoomCount) gatewayRoomCount.textContent = formatNumber(roomCount);
    if (gatewayMessageCount) gatewayMessageCount.textContent = formatNumber(messageCount);
    if (gatewayLastSync) gatewayLastSync.textContent = syncLabel + (hasGateway ? ' · Gateway' : ' · 本地');
    if (gatewayUptime) {
      gatewayUptime.textContent = (hasGateway && !partialGateway && uptimeSec)
        ? Math.floor(uptimeSec / 3600) + 'h ' + Math.floor((uptimeSec % 3600) / 60) + 'm ' + (uptimeSec % 60) + 's'
        : '--';
    }
    if (gatewayStateVersion) {
      gatewayStateVersion.textContent = (hasGateway && !partialGateway && summary && summary.state_version)
        ? summary.state_version
        : '--';
    }
    updateGatewayConnectionTag(partialGateway ? '部分同步' : (hasGateway ? '已连接' : '本地预览'), partialGateway ? 'warning' : (hasGateway ? 'success' : 'default'));
    updateAlertCounts();
  }

  function renderDashboardEvents(auditPayload) {
    var eventList = document.querySelector('.ds-event-list');
    if (!eventList) return;
    var events = (auditPayload && Array.isArray(auditPayload.events)) ? auditPayload.events : [];
    clear(eventList);
    if (!events.length) {
      eventList.appendChild(el('div', { class: 'ds-event-item' },
        el('span', { class: 'ds-event-text', style: 'color:var(--ds-text-secondary);' }, '暂无系统事件')
      ));
      return;
    }
    var typeMap = {
      'admin:ban_resident': { label: '封禁', cls: 'warn' },
      'admin:unban_resident': { label: '解封', cls: 'info' },
      'admin:freeze_room': { label: '冻结', cls: 'warn' },
      'admin:unfreeze_room': { label: '解冻', cls: 'info' },
      'admin:moderate_message': { label: '审核', cls: 'warn' },
      'admin:config': { label: '配置', cls: 'info' },
      'shell:message': { label: '消息', cls: 'info' },
      'auth:login': { label: '登录', cls: 'info' },
      'auth:logout': { label: '登出', cls: 'info' }
    };
    var recent = events.slice(0, 10);
    for (var i = 0; i < recent.length; i++) {
      var ev = recent[i];
      var time = new Date(ev.timestamp_ms).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
      var meta = typeMap[ev.action] || { label: ev.action, cls: 'info' };
      var desc = meta.label + ' · ' + (ev.target || '') + (ev.actor_id ? ' by ' + ev.actor_id : '');
      eventList.appendChild(el('div', { class: 'ds-event-item' },
        el('span', { class: 'ds-event-time' }, time),
        el('span', { class: 'ds-event-text' }, desc),
        el('span', { class: 'ds-event-type ' + meta.cls }, meta.label)
      ));
    }
  }

  function roleFromGatewayRoles(roles) {
    if (!Array.isArray(roles)) return 'resident';
    if (roles.indexOf('admin') !== -1 || roles.indexOf('steward') !== -1 || roles.indexOf('owner') !== -1) return 'admin';
    if (roles.indexOf('guest') !== -1) return 'guest';
    return 'resident';
  }

  function normalizeGatewayResidents(payload) {
    if (!Array.isArray(payload)) return [];
    return payload.map(function (item, index) {
      var id = String(item.resident_id || item.id || 'resident-' + (index + 1));
      var roles = Array.isArray(item.roles) ? item.roles : [];
      var cityCount = Array.isArray(item.active_cities) ? item.active_cities.length : 0;
      var online = item.online === true;
      var registrationState = String(item.registration_state || 'unknown').toLowerCase();
      var banned = item.is_banned === true || registrationState === 'suspended';
      var lastSeenMs = item.last_seen_at_ms;
      var lastSeenText = '网关同步';
      if (lastSeenMs) {
        var secondsAgo = Math.floor((Date.now() - lastSeenMs) / 1000);
        if (secondsAgo < 60) lastSeenText = '刚刚';
        else if (secondsAgo < 3600) lastSeenText = Math.floor(secondsAgo / 60) + ' 分钟前';
        else if (secondsAgo < 86400) lastSeenText = Math.floor(secondsAgo / 3600) + ' 小时前';
        else lastSeenText = Math.floor(secondsAgo / 86400) + ' 天前';
      }
      return {
        id: id,
        nick: item.nickname || id,
        email: item.email_masked || (item.avatar_id || id) + '@resident.local',
        registrationState: registrationState,
        createdAtMs: Number(item.created_at_ms || 0),
        verifiedAtMs: Number(item.verified_at_ms || 0),
        lastLoginAtMs: Number(item.last_login_at_ms || 0),
        role: roleFromGatewayRoles(roles),
        status: banned ? 'banned' : (online ? 'online' : 'offline'),
        lastSeen: lastSeenText,
        msgCount: cityCount
      };
    });
  }

  function roomTypeFromGateway(room) {
    if (room.kind === 'direct' || room.scope === 'private') return 'private';
    if (room.scope === 'world' || String(room.conversation_id || room.id || '').indexOf('room:world:') === 0) return 'world';
    return 'group';
  }

  function normalizeGatewayRooms(shellState) {
    var source = shellState?.conversation_shell?.conversations || shellState?.rooms || [];
    if (!Array.isArray(source)) return [];
    return source.map(function (room, index) {
      var id = String(room.conversation_id || room.id || 'room-' + (index + 1));
      var msgs = Array.isArray(room.messages) ? room.messages : [];
      return {
        id: id,
        name: room.title || room.thread_headline || id,
        type: roomTypeFromGateway(room),
        members: Number(room.member_count || 0),
        todayMsg: msgs.length,
        unread: Number(room.unread_count || 0),
        creator: room.participant_label || room.self_label || room.peer_label || 'gateway',
        created: room.activity_time_label || room.last_activity_label || '网关同步',
        frozen: Boolean(room.frozen),
        image_layer: room.image_layer || room.scene_image || null,
        hotspot_layer: room.hotspot_layer || null
      };
    });
  }

  function normalizeGatewayMessages(shellState) {
    var source = shellState?.conversation_shell?.conversations || shellState?.rooms || [];
    if (!Array.isArray(source)) return [];
    var out = [];
    for (var i = 0; i < source.length; i++) {
      var room = source[i];
      var roomId = String(room.conversation_id || room.id || '');
      var roomTitle = room.title || room.thread_headline || room.conversation_id || room.id || '网关会话';
      var msgs = Array.isArray(room.messages) ? room.messages : [];
      for (var j = 0; j < msgs.length; j++) {
        var msg = msgs[j];
        var status = msg.delivery_status === 'failed' ? 'flagged' : 'passed';
        if (msg.moderation_status === 'approved') status = 'approved';
        else if (msg.moderation_status === 'blocked') status = 'blocked';
        else if (msg.moderation_status === 'handled') status = 'handled';
        out.push({
          message_id: msg.message_id || '',
          conversation_id: roomId,
          time: msg.timestamp_label || '网关同步',
          sender: msg.sender || 'unknown',
          room: '#' + roomTitle,
          content: msg.is_recalled ? '消息已撤回' : (msg.text || ''),
          status: status
        });
      }
    }
    return out.slice(-80).reverse();
  }

  async function loadGatewayAdminData() {
    if (!gatewayUrl) {
      setGatewayStatus('Gateway 未连接', 'warning');
      updateDashboardSummary('local');
      return;
    }
    setGatewayStatus('Gateway 同步中', 'info');
    setSectionLoading('mod-residents', true);
    setSectionLoading('mod-rooms', true);
    setSectionLoading('mod-messages', true);
    var fetchFailed = false;
    try {
      var identity = encodeURIComponent(currentGatewayIdentity());
      var results = await Promise.allSettled([
        fetchGatewayJson('/v1/admin/residents'),
        fetchGatewayJson('/v1/shell/state?resident_id=' + identity),
        fetchGatewayJson('/v1/admin/summary'),
        fetchGatewayJson('/v1/admin/audit-log')
      ]);
      var residentPayload = results[0].status === 'fulfilled' ? results[0].value : null;
      var shellPayload = results[1].status === 'fulfilled' ? results[1].value : null;
      var summaryPayload = results[2].status === 'fulfilled' ? results[2].value : null;
      var auditPayload = results[3].status === 'fulfilled' ? results[3].value : null;
      var residentReadOk = results[0].status === 'fulfilled' && Array.isArray(residentPayload);
      var shellReadOk = results[1].status === 'fulfilled' && shellPayload && typeof shellPayload === 'object' && (
        Array.isArray(shellPayload.rooms) ||
        (shellPayload.conversation_shell && Array.isArray(shellPayload.conversation_shell.conversations))
      );
      var summaryReadOk = results[2].status === 'fulfilled' && summaryPayload &&
        typeof summaryPayload.resident_count === 'number' &&
        typeof summaryPayload.room_count === 'number' &&
        typeof summaryPayload.message_count === 'number' &&
        typeof summaryPayload.online_count === 'number';
      var auditReadOk = results[3].status === 'fulfilled' && auditPayload && Array.isArray(auditPayload.events);
      var anyRejected = results.some(function (result) { return result.status === 'rejected'; });
      fetchFailed = anyRejected || !residentReadOk || !shellReadOk || !summaryReadOk || !auditReadOk;
      // Gateway 是正式真源：有效空数组也必须覆盖旧本地 mock，失败/畸形响应则显示空态。
      residents = residentReadOk ? normalizeGatewayResidents(residentPayload) : [];
      rooms = shellReadOk ? normalizeGatewayRooms(shellPayload) : [];
      messages = shellReadOk ? normalizeGatewayMessages(shellPayload) : [];
      renderResidents('all', 'all', '');
      renderRooms('all', '');
      renderMessages('all', 'all', '');
      updateDashboardSummary(fetchFailed ? 'gateway-partial' : 'gateway', summaryPayload);
      renderDashboardEvents(auditPayload);
      setGatewayStatus(fetchFailed ? 'Gateway 部分读取失败' : 'Gateway 在线', fetchFailed ? 'warning' : 'online');
      // Preload secondary data types so modules render from Gateway data on first visit
      loadInviteCodes().catch(function () {});
      loadPermissionGroups().catch(function () {});
      loadSysConfig().catch(function () {});
      loadAuditLog().catch(function () {});
    } catch (error) {
      console.warn('admin-ds gateway sync failed', error);
      residents = [];
      rooms = [];
      messages = [];
      renderResidents('all', 'all', '');
      renderRooms('all', '');
      renderMessages('all', 'all', '');
      updateDashboardSummary('gateway-partial');
      setGatewayStatus('Gateway 读取失败', 'warning');
    } finally {
      setSectionLoading('mod-residents', false);
      setSectionLoading('mod-rooms', false);
      setSectionLoading('mod-messages', false);
    }
  }

  async function loadInviteCodes() {
    if (!gatewayUrl) return;
    try {
      var result = await fetchGatewayJson('/v1/admin/invites');
      if (Array.isArray(result)) {
        inviteCodes = result.map(function(ic) {
          var expired = ic.max_uses > 0 && ic.used_count >= ic.max_uses;
          return {
            code: ic.code,
            room: '-',
            maxUses: ic.max_uses,
            used: ic.used_count,
            expires: ic.revoked ? '已作废' : (expired ? '已用尽' : '有效'),
            creator: ic.created_by,
            status: ic.revoked ? 'revoked' : (expired ? 'expired' : 'active')
          };
        });
      } else {
        // Gateway 已连接时，空/失败响应必须展示真实空态，不能回退到本地 mock。
        inviteCodes = [];
        showAdminNotice('Gateway 邀请码读取失败，已显示空态', 'error');
      }
    } catch (e) {
      console.warn('admin-ds load invites failed', e);
      inviteCodes = [];
      showAdminNotice('Gateway 邀请码读取失败，已显示空态', 'error');
    }
    renderInvites();
  }

  async function loadPermissionGroups() {
    if (!gatewayUrl) return;
    try {
      var result = await fetchGatewayJson('/v1/admin/permission-groups');
      if (Array.isArray(result)) {
        permissionGroups = result;
      } else {
        permissionGroups = [];
        showAdminNotice('Gateway 权限组读取失败，已显示空态', 'error');
      }
    } catch (e) {
      console.warn('admin-ds load permission groups failed', e);
      permissionGroups = [];
      showAdminNotice('Gateway 权限组读取失败，已显示空态', 'error');
    }
    renderPermissionGroups();
  }

  async function loadCapabilities() {
    if (!gatewayUrl) return [];
    try {
      var result = await fetchGatewayJson('/v1/admin/capabilities');
      if (Array.isArray(result)) { return result; }
    } catch (e) { console.warn('admin-ds load capabilities failed', e); }
    return [];
  }

  function builtinPermissionGroups() {
    return [
      { id: 'builtin-admin', name: '管理员', description: '全部权限 · 可管理居民、房间、消息审核', capabilities: [] },
      { id: 'builtin-resident', name: '正式居民', description: '可创建房间、发送消息、邀请他人', capabilities: [] },
      { id: 'builtin-restricted', name: '受限居民', description: '仅可加入已有房间、发送消息需审核', capabilities: [] },
      { id: 'builtin-guest', name: '访客', description: '仅可浏览世界广场、不可私聊', capabilities: [] }
    ];
  }

  function permissionGroupItems() {
    if (gatewayUrl) return permissionGroups;
    return permissionGroups.length ? permissionGroups : builtinPermissionGroups();
  }

  function renderPermissionGroups() {
    var container = document.getElementById('permissionGroupList');
    if (!container) return;
    clear(container);

    var groups = permissionGroupItems();
    if (!groups.length) {
      container.appendChild(el('p', { style: 'color:var(--ds-text-secondary);' }, 'Gateway 暂无已创建权限组'));
      return;
    }

    // Count people per group
    var counts = {};
    groups.forEach(function(g) { counts[g.id] = 0; });

    for (var i = 0; i < groups.length; i++) {
      var g = groups[i];
      var row = el('div');
      row.style.cssText = 'display:flex;justify-content:space-between;align-items:center;padding:10px 0;border-bottom:1px solid var(--ds-border-light);';
      var left = el('div');
      var nameEl = el('strong');
      nameEl.textContent = g.name;
      left.appendChild(nameEl);
      var descEl = el('div');
      descEl.style.cssText = 'color:var(--ds-text-muted);font-size:12px;';
      descEl.textContent = g.description;
      left.appendChild(descEl);

      // Show capability tags for custom groups
      if (g.capabilities && g.capabilities.length) {
        var capRow = el('div');
        capRow.style.cssText = 'margin-top:4px;';
        g.capabilities.forEach(function(cap) {
          var tag = el('span');
          tag.className = 'ds-tag default';
          tag.style.cssText = 'margin-right:4px;font-size:11px;';
          tag.textContent = cap;
          capRow.appendChild(tag);
        });
        left.appendChild(capRow);
      }

      row.appendChild(left);
      var tagEl = el('span');
      tagEl.className = 'ds-tag ' + (g.capabilities && g.capabilities.length ? 'info' : 'default');
      tagEl.textContent = (counts[g.id] || 0) + ' 人';
      row.appendChild(tagEl);
      container.appendChild(row);
    }
  }

  // ====== Module Switching ======

  function switchModule(moduleName) {
    activeModule = moduleName;
    var items = document.querySelectorAll('.ds-nav-item');
    for (var i = 0; i < items.length; i++) {
      items[i].classList.toggle('active', items[i].dataset.module === moduleName);
    }
    var modules = document.querySelectorAll('.ds-module');
    for (var j = 0; j < modules.length; j++) {
      modules[j].classList.remove('active');
    }
    var target = document.getElementById('mod-' + moduleName);
    if (target) target.classList.add('active');
    closeDetail();
    if (window.innerWidth <= 768) { collapseSidebar(); }
    if (moduleName === 'sysconfig') { loadSysConfig(); }
    if (moduleName === 'permissions') { loadInviteCodes(); loadPermissionGroups(); }
    if (moduleName === 'logs') { setSectionLoading('mod-logs', true); loadAuditLog().finally(function () { setSectionLoading('mod-logs', false); }); }
    if (moduleName === 'world-notices') { loadWorldNotices(); }
    if (moduleName === 'safety-advisories') { loadSafetyData(); }
    if (moduleName === 'scene') { loadSceneModule(); }
    if (moduleName === 'devices') { loadDevices(); }
  }

  var navItems = document.querySelectorAll('.ds-nav-item');
  for (var ni = 0; ni < navItems.length; ni++) {
    navItems[ni].addEventListener('click', function () {
      switchModule(this.dataset.module);
    });
  }

  // ====== Sidebar Toggle ======

  function collapseSidebar() {
    sidebar.classList.add('collapsed');
    sidebarOverlay.classList.remove('show');
    sidebarExpanded = false;
  }

  function expandSidebar() {
    sidebar.classList.remove('collapsed');
    sidebarExpanded = true;
  }

  sidebarToggle.addEventListener('click', function () {
    if (window.innerWidth <= 768) {
      if (sidebarExpanded) { collapseSidebar(); }
      else { expandSidebar(); sidebarOverlay.classList.add('show'); }
    } else {
      if (sidebarExpanded) { collapseSidebar(); }
      else { expandSidebar(); }
    }
  });

  sidebarOverlay.addEventListener('click', function () { collapseSidebar(); });

  function handleResize() {
    if (window.innerWidth <= 768) {
      if (sidebarExpanded && !sidebarOverlay.classList.contains('show')) { collapseSidebar(); }
    }
  }
  window.addEventListener('resize', handleResize);
  handleResize();

  // ====== Detail Panel ======

  /* openDetail(title, buildBody, buildActions)
     - buildBody(container): 接收 detailBody 容器，往里面 append DOM
     - buildActions(container): 接收 detailActions 容器，往里面 append DOM（可选） */
  function openDetail(title, buildBody, buildActions) {
    detailTitle.textContent = title;
    clear(detailBody);
    if (buildBody) buildBody(detailBody);
    detailPanel.classList.remove('hidden');
    clear(detailActions);
    if (buildActions) {
      buildActions(detailActions);
      detailActions.style.display = 'flex';
    } else {
      detailActions.style.display = 'none';
    }
  }

  function closeDetail() {
    detailPanel.classList.add('hidden');
    var selected = document.querySelectorAll('.ds-table tbody tr.selected');
    for (var s = 0; s < selected.length; s++) { selected[s].classList.remove('selected'); }
  }

  detailClose.addEventListener('click', closeDetail);

  // ====== Detail field helper ======

  function detailField(label, valueEl) {
    var field = el('div', { class: 'ds-detail-field' });
    var lbl = el('div', { class: 'ds-detail-label' }, label);
    var val = el('div', { class: 'ds-detail-value' });
    if (typeof valueEl === 'string') { val.textContent = valueEl; }
    else { val.appendChild(valueEl); }
    field.appendChild(lbl);
    field.appendChild(val);
    return field;
  }

  function detailFieldStyled(label, valueStr, styleCss) {
    var field = el('div', { class: 'ds-detail-field' });
    field.appendChild(el('div', { class: 'ds-detail-label' }, label));
    var val = el('div', { class: 'ds-detail-value', style: styleCss }, valueStr);
    field.appendChild(val);
    return field;
  }

  // ====== Status / Tag helpers ======

  function makeTag(text, tagClass) {
    return el('span', { class: 'ds-tag ' + tagClass }, text);
  }

  function makeStatusDot(text, statusClass) {
    return el('span', { class: 'ds-status-indicator ' + statusClass }, text);
  }

  function makeBtn(text, btnClass) {
    return el('button', { class: 'ds-btn ' + btnClass, type: 'button' }, text);
  }

  function makeBtnGroup() {
    return el('div', { class: 'ds-btn-group' });
  }

  // ====== 共享热点列表编辑器（2026-08-02 去重） ======
  // 房间详情内联版（layout:'blocks'，admin-hotspot-* 容器类）与场景模块页
  // （layout:'flex'，内联 flex 行）共用同一套行渲染/收集/载荷逻辑。
  var HOTSPOT_COORD_FIELDS = [
    { key: 'x_permyriad', placeholder: 'X', def: 2500 },
    { key: 'y_permyriad', placeholder: 'Y', def: 2500 },
    { key: 'width_permyriad', placeholder: 'W', def: 800 },
    { key: 'height_permyriad', placeholder: 'H', def: 600 }
  ];

  function newHotspotDefaults() {
    return {
      hotspot_id: 'hotspot-' + Date.now(),
      label: '新热点',
      interaction_hint: '',
      sprite_hint: 'default',
      x_permyriad: 2500,
      y_permyriad: 2500,
      width_permyriad: 800,
      height_permyriad: 600
    };
  }

  function createHotspotListEditor(existingHotspots, options) {
    options = options || {};
    var layout = options.layout === 'flex' ? 'flex' : 'blocks';
    var onRowsRendered = typeof options.onRowsRendered === 'function' ? options.onRowsRendered : null;
    var hotspotList = layout === 'flex'
      ? el('div', { style: 'padding:0 1rem;' })
      : el('div', { class: 'admin-hotspot-list' });

    function renderRows() {
      clear(hotspotList);
      for (var hi = 0; hi < existingHotspots.length; hi++) {
        (function (hs, idx) {
          var row, fields;
          if (layout === 'flex') {
            row = el('div', { style: 'display:flex;gap:0.5rem;align-items:center;padding:0.5rem 0;border-bottom:1px solid var(--ds-border-light);flex-wrap:wrap;' });
            fields = row;
          } else {
            row = el('div', { class: 'admin-hotspot-row' });
            fields = el('div', { class: 'admin-hotspot-fields' });
          }
          var textWidths = layout === 'flex' ? ['width:100px;', 'width:100px;', 'width:140px;'] : [null, null, null];
          var textClasses = layout === 'flex' ? ['ds-input', 'ds-input', 'ds-input'] : ['ds-input admin-hotspot-id', 'ds-input admin-hotspot-label', 'ds-input admin-hotspot-hint'];
          var textDefs = [
            { placeholder: 'ID', value: hs.hotspot_id || '' },
            { placeholder: '标签', value: hs.label || '' },
            { placeholder: '交互提示', value: hs.interaction_hint || '' }
          ];
          var textInputs = [];
          for (var ti = 0; ti < textDefs.length; ti++) {
            var textProps = { type: 'text', class: textClasses[ti], placeholder: textDefs[ti].placeholder, value: textDefs[ti].value };
            if (textWidths[ti]) textProps.style = textWidths[ti];
            var textInput = el('input', textProps);
            textInputs.push(textInput);
            fields.appendChild(textInput);
          }

          var coordsContainer = layout === 'flex' ? row : el('div', { class: 'admin-hotspot-coords' });
          var coordRefs = [];
          for (var ci = 0; ci < HOTSPOT_COORD_FIELDS.length; ci++) {
            var coordField = HOTSPOT_COORD_FIELDS[ci];
            var coordInput = el('input', {
              type: 'number', class: layout === 'flex' ? 'ds-input' : 'ds-input admin-hotspot-coord',
              placeholder: coordField.placeholder,
              value: String(typeof hs[coordField.key] === 'number' ? hs[coordField.key] : coordField.def),
              style: 'width:58px;'
            });
            coordRefs.push({ input: coordInput, key: coordField.key, def: coordField.def });
            coordsContainer.appendChild(coordInput);
          }
          if (layout !== 'flex') fields.appendChild(coordsContainer);

          var delBtn = makeBtn('删除', 'ds-btn-danger-text ds-btn-xs');
          delBtn.addEventListener('click', function () {
            existingHotspots.splice(idx, 1);
            renderRows();
          });
          if (layout !== 'flex') row.appendChild(fields);
          row.appendChild(delBtn);
          hotspotList.appendChild(row);

          row._coordInputs = coordRefs;
          row._textInputs = textInputs;
        })(existingHotspots[hi], hi);
      }
      if (!existingHotspots.length) {
        hotspotList.appendChild(el('div', { class: 'admin-hotspot-empty', style: 'color:var(--ds-text-secondary);font-size:12px;padding:8px 0;' }, '暂无热点'));
      }
      if (onRowsRendered) onRowsRendered();
    }

    function collectHotspots() {
      var hotspotsOut = [];
      var allRows = hotspotList.children;
      for (var ri = 0; ri < allRows.length; ri++) {
        var r = allRows[ri];
        if (!r._textInputs) continue;
        var ids = r._textInputs;
        var cs = r._coordInputs;
        var h = {
          hotspot_id: (ids && ids[0]) ? ids[0].value : ('hotspot-' + ri),
          label: (ids && ids[1]) ? ids[1].value : '',
          sprite_hint: 'default',
          interaction_hint: (ids && ids[2]) ? ids[2].value : ''
        };
        for (var ci2 = 0; ci2 < (cs ? cs.length : 0); ci2++) {
          var c = cs[ci2];
          h[c.key] = parseInt(c.input.value, 10) || c.def;
        }
        if (!h.x_permyriad) h.x_permyriad = 2500;
        if (!h.y_permyriad) h.y_permyriad = 2500;
        if (!h.width_permyriad) h.width_permyriad = 800;
        if (!h.height_permyriad) h.height_permyriad = 600;
        hotspotsOut.push(h);
      }
      return hotspotsOut;
    }

    renderRows();
    return {
      listEl: hotspotList,
      renderRows: renderRows,
      collectHotspots: collectHotspots,
      addHotspot: function () {
        existingHotspots.push(newHotspotDefaults());
        renderRows();
      }
    };
  }

  function buildHotspotLayerPayload(hotspotsOut) {
    if (!hotspotsOut || !hotspotsOut.length) return null;
    return {
      layer_id: 'admin-hotspot-' + Date.now(),
      coordinate_system: 'scene-permyriad',
      owner_editable: true,
      hotspots: hotspotsOut
    };
  }

  function buildImageLayerPayload(selectedPreset, dayUrl, nightUrl, layerIdPrefix) {
    if (!selectedPreset && !dayUrl && !nightUrl) return null;
    return {
      layer_id: layerIdPrefix + Date.now(),
      preset: selectedPreset || 'custom',
      asset_hint: selectedPreset || 'custom',
      aspect_ratio_permyriad: 5625,
      owner_editable: true,
      day_image_url: dayUrl || null,
      night_image_url: nightUrl || null
    };
  }

  function ensureAdminNotice() {
    var notice = document.getElementById('dsAdminNotice');
    if (notice) return notice;
    notice = el('div', { id: 'dsAdminNotice', class: 'ds-admin-notice', role: 'status', 'aria-live': 'polite' });
    var content = document.getElementById('dsContent');
    if (content) content.insertBefore(notice, content.firstChild);
    return notice;
  }

  function showAdminNotice(text, tone) {
    var notice = ensureAdminNotice();
    notice.textContent = text;
    notice.className = 'ds-admin-notice show ' + (tone || 'info');
    window.clearTimeout(showAdminNotice._timer);
    showAdminNotice._timer = window.setTimeout(function () {
      notice.classList.remove('show');
    }, 2600);
  }

  function markUnavailableButton(button, reason) {
    if (!button) return button;
    button.disabled = true;
    button.setAttribute('aria-disabled', 'true');
    button.setAttribute('title', reason);
    button.dataset.disabledReason = reason;
    return button;
  }

  function copyText(text, successMessage) {
    var value = String(text || '');
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(value).then(function () {
        showAdminNotice(successMessage || '已复制', 'success');
      }).catch(function () {
        fallbackCopyText(value, successMessage);
      });
      return;
    }
    fallbackCopyText(value, successMessage);
  }

  function fallbackCopyText(text, successMessage) {
    var input = el('textarea', { style: 'position:fixed;left:-9999px;top:-9999px;' }, text);
    document.body.appendChild(input);
    input.select();
    try {
      document.execCommand('copy');
      showAdminNotice(successMessage || '已复制', 'success');
    } catch (_) {
      showAdminNotice('复制失败，请手动复制', 'warning');
    }
    document.body.removeChild(input);
  }

  function csvEscape(value) {
    var text = String(value == null ? '' : value);
    return '"' + text.replace(/"/g, '""') + '"';
  }

  function downloadCsv(filename, columns, rows) {
    var headerCells = [];
    for (var h = 0; h < columns.length; h++) {
      headerCells.push(csvEscape(columns[h].label));
    }
    var bodyLines = [];
    for (var r = 0; r < rows.length; r++) {
      var row = rows[r];
      var cells = [];
      for (var c = 0; c < columns.length; c++) {
        var col = columns[c];
        cells.push(csvEscape(typeof col.get === 'function' ? col.get(row) : row[col.key]));
      }
      bodyLines.push(cells.join(','));
    }
    var header = headerCells.join(',');
    var body = bodyLines.join('\n');
    var blob = new Blob(['\uFEFF' + header + '\n' + body], { type: 'text/csv;charset=utf-8' });
    var url = URL.createObjectURL(blob);
    var link = el('a', { href: url, download: filename });
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.setTimeout(function () { URL.revokeObjectURL(url); }, 0);
    showAdminNotice('已导出 ' + rows.length + ' 条数据', 'success');
  }

  function filteredResidents() {
    var status = document.getElementById('residentStatusFilter').value;
    var role = document.getElementById('residentRoleFilter').value;
    var searchTerm = document.getElementById('residentSearch').value.trim().toLowerCase();
    return residents.filter(function (r) {
      if (status !== 'all' && r.status !== status) return false;
      if (role !== 'all' && r.role !== role) return false;
      if (!searchTerm) return true;
      return r.id.toLowerCase().indexOf(searchTerm) !== -1 ||
        r.nick.toLowerCase().indexOf(searchTerm) !== -1 ||
        r.email.toLowerCase().indexOf(searchTerm) !== -1;
    });
  }

  function filteredLogs() {
    var level = document.getElementById('logLevelFilter').value;
    var type = document.getElementById('logTypeFilter').value;
    var searchTerm = document.getElementById('logSearch').value.trim().toLowerCase();
    return logs.filter(function (item) {
      if (level !== 'all' && item.level !== level) return false;
      if (type !== 'all' && item.type !== type) return false;
      if (!searchTerm) return true;
      return item.desc.toLowerCase().indexOf(searchTerm) !== -1 ||
        item.source.toLowerCase().indexOf(searchTerm) !== -1;
    });
  }

  function openResidentSessions(resident) {
    switchModule('rooms');
    window.setTimeout(function () {
      var searchInput = document.getElementById('roomSearch');
      if (searchInput) {
        searchInput.value = resident.nick;
        renderRooms(document.getElementById('roomTypeFilter').value, resident.nick);
        showAdminNotice('已筛选 ' + resident.nick + ' 相关会话', 'info');
      }
    }, 80);
  }

  function makeLogLevel(text, level) {
    return el('span', { class: 'ds-log-level ' + level }, text);
  }

  function makeTd(text, styleCss) {
    var td = el('td');
    if (styleCss) td.style.cssText = styleCss;
    if (typeof text === 'string') { td.textContent = text; }
    else { td.appendChild(text); }
    return td;
  }

  function makeTdMono(text) {
    var span = el('span', { style: 'font-family:var(--ds-font-mono);font-size:12px;' }, text);
    return makeTd(span);
  }

  // ====== Render Residents Table ======

  function renderResidents(filterStatus, filterRole, searchTerm) {
    var tbody = document.getElementById('residentTableBody');
    var filtered = residents.filter(function (r) {
      if (filterStatus && filterStatus !== 'all' && r.status !== filterStatus) return false;
      if (filterRole && filterRole !== 'all' && r.role !== filterRole) return false;
      if (searchTerm) {
        var term = searchTerm.toLowerCase();
        if (r.id.toLowerCase().indexOf(term) === -1 &&
            r.nick.toLowerCase().indexOf(term) === -1 &&
            r.email.toLowerCase().indexOf(term) === -1) return false;
      }
      return true;
    });

    clear(tbody);

    if (!filtered.length) { renderEmptyRow(tbody, 8, searchTerm ? '没有匹配的居民' : '暂无居民数据'); renderPagination('residents', 0, function(p){ renderResidents(filterStatus, filterRole, searchTerm); }); return; }

    var residentPage = paginateArray(filtered, pageState.residents || 1);
    for (var i = 0; i < residentPage.length; i++) {
      (function (resident) {
        var tr = el('tr', { data: { residentId: resident.id } });

        tr.appendChild(makeTdMono(resident.id));

        var tdNick = el('td');
        tdNick.appendChild(el('strong', null, resident.nick));
        tr.appendChild(tdNick);

        tr.appendChild(makeTd(resident.email, 'color:var(--ds-text-secondary);'));

        var tdRole = el('td');
        tdRole.appendChild(makeTag(L.roleText[resident.role] || resident.role, L.roleTag[resident.role] || 'default'));
        tr.appendChild(tdRole);

        var tdStatus = el('td');
        var sc = L.statusClass[resident.status] || 'offline';
        var st = L.statusText[resident.status] || resident.status;
        tdStatus.appendChild(makeStatusDot(st, sc));
        tr.appendChild(tdStatus);

        tr.appendChild(makeTd(resident.lastSeen, 'color:var(--ds-text-secondary);'));

        tr.appendChild(makeTd(resident.msgCount.toLocaleString()));

        // 操作按钮
        var tdActions = el('td');
        var btnGroup = makeBtnGroup();

        if (resident.status === 'banned') {
          var restoreBtn = makeBtn('恢复', 'ds-btn-outline ds-btn-xs');
          restoreBtn.addEventListener('click', function (e) { e.stopPropagation(); unbanResident(resident.id, restoreBtn); });
          btnGroup.appendChild(restoreBtn);
        } else {
          var banBtn = makeBtn('禁用', 'ds-btn-outline ds-btn-xs');
          banBtn.addEventListener('click', function (e) { e.stopPropagation(); banResident(resident.id, banBtn); });
          btnGroup.appendChild(banBtn);
        }

        var sessionBtn = makeBtn('会话', 'ds-btn-outline ds-btn-xs');
        sessionBtn.addEventListener('click', function (e) {
          e.stopPropagation();
          openResidentSessions(resident);
        });
        btnGroup.appendChild(sessionBtn);

        tdActions.appendChild(btnGroup);
        tr.appendChild(tdActions);

        // 行点击 → 详情
        tr.addEventListener('click', function (e) {
          if (e.target.closest('button')) return;
          var prev = tbody.querySelectorAll('tr.selected');
          for (var p = 0; p < prev.length; p++) { prev[p].classList.remove('selected'); }
          tr.classList.add('selected');

          var sc2 = L.statusClass[resident.status] || 'offline';
          var st2 = L.statusText[resident.status] || resident.status;
          var rt2 = L.roleText[resident.role] || resident.role;

          openDetail(
            '居民: ' + resident.nick,
            function (container) {
              container.appendChild(detailFieldStyled('居民 ID', resident.id, 'font-family:var(--ds-font-mono);'));
              container.appendChild(detailField('昵称', resident.nick));
              container.appendChild(detailField('登录邮箱', resident.email));
              container.appendChild(detailField('角色', rt2));
              container.appendChild(detailField('状态', makeStatusDot(st2, sc2)));
              container.appendChild(detailField('最近在线', resident.lastSeen));
              container.appendChild(detailField('累计消息', resident.msgCount.toLocaleString()));
              container.appendChild(detailField('注册状态', resident.registrationState === 'suspended' ? '已暂停' : (resident.registrationState === 'active' ? '正常' : '无注册记录')));
              container.appendChild(detailField('注册时间', resident.createdAtMs ? new Date(resident.createdAtMs).toLocaleString('zh-CN') : '-'));
              container.appendChild(detailField('验证时间', resident.verifiedAtMs ? new Date(resident.verifiedAtMs).toLocaleString('zh-CN') : '-'));
              container.appendChild(detailField('最近登录', resident.lastLoginAtMs ? new Date(resident.lastLoginAtMs).toLocaleString('zh-CN') : '-'));
            },
            function (actions) {
              var viewSessionsBtn = makeBtn('查看会话', 'ds-btn-outline ds-btn-sm');
              viewSessionsBtn.addEventListener('click', function () { openResidentSessions(resident); });
              actions.appendChild(viewSessionsBtn);
              if (resident.status === 'banned') {
                var detailRestore = makeBtn('恢复居民', 'ds-btn-primary ds-btn-sm');
                detailRestore.addEventListener('click', function () { closeDetail(); unbanResident(resident.id, detailRestore); });
                actions.appendChild(detailRestore);
              } else {
                var detailBan = makeBtn('禁用居民', 'ds-btn-danger-text ds-btn-sm');
                detailBan.addEventListener('click', function () { closeDetail(); banResident(resident.id, detailBan); });
                actions.appendChild(detailBan);
              }
            }
          );
        });

        tbody.appendChild(tr);
      })(residentPage[i]);
    }

    renderPagination('residents', filtered.length, function(p) { renderResidents(filterStatus, filterRole, searchTerm); });
  }

  // Resident search/filter bindings
  document.getElementById('residentSearch').addEventListener('input', function () {
    renderResidents(
      document.getElementById('residentStatusFilter').value,
      document.getElementById('residentRoleFilter').value,
      this.value
    );
  });
  document.getElementById('residentStatusFilter').addEventListener('change', function () {
    renderResidents(this.value, document.getElementById('residentRoleFilter').value, document.getElementById('residentSearch').value);
  });
  document.getElementById('residentRoleFilter').addEventListener('change', function () {
    renderResidents(document.getElementById('residentStatusFilter').value, this.value, document.getElementById('residentSearch').value);
  });

  // ====== Render Rooms Table ======

  function renderRooms(filterType, searchTerm) {
    var tbody = document.getElementById('roomTableBody');
    var filtered = rooms.filter(function (r) {
      if (filterType && filterType !== 'all' && r.type !== filterType) return false;
      if (searchTerm) {
        var term = searchTerm.toLowerCase();
        if (r.id.toLowerCase().indexOf(term) === -1 &&
            r.name.toLowerCase().indexOf(term) === -1 &&
            r.creator.toLowerCase().indexOf(term) === -1) return false;
      }
      return true;
    });

    clear(tbody);

    if (!filtered.length) { renderEmptyRow(tbody, 7, searchTerm ? '没有匹配的会话' : '暂无会话数据'); renderPagination('rooms', 0, function(p){ renderRooms(filterType, searchTerm); }); return; }

    var roomPage = paginateArray(filtered, pageState.rooms || 1);
    for (var i = 0; i < roomPage.length; i++) {
      (function (room) {
        var tr = el('tr', { data: { roomId: room.id } });

        tr.appendChild(makeTdMono(room.id));

        var tdName = el('td');
        tdName.appendChild(el('strong', null, room.name));
        tr.appendChild(tdName);

        var tdType = el('td');
        tdType.appendChild(makeTag(L.roomTypeText[room.type] || room.type, L.roomTypeTag[room.type] || 'default'));
        tr.appendChild(tdType);

        tr.appendChild(makeTd(String(room.members)));
        tr.appendChild(makeTd(room.todayMsg.toLocaleString()));
        tr.appendChild(makeTd(String(room.unread)));
        tr.appendChild(makeTd(room.creator));
        tr.appendChild(makeTd(room.created, 'color:var(--ds-text-secondary);'));

        // 行点击 → 详情
        tr.addEventListener('click', function () {
          var prev = tbody.querySelectorAll('tr.selected');
          for (var p = 0; p < prev.length; p++) { prev[p].classList.remove('selected'); }
          tr.classList.add('selected');

          var rtt = L.roomTypeText[room.type] || room.type;

          openDetail(
            '房间: ' + room.name,
            function (container) {
              container.appendChild(detailFieldStyled('房间 ID', room.id, 'font-family:var(--ds-font-mono);'));
              container.appendChild(detailField('房间名', room.name));
              container.appendChild(detailField('类型', rtt));
              container.appendChild(detailField('成员数', room.members + ' 人'));
              container.appendChild(detailField('今日消息', room.todayMsg.toLocaleString() + ' 条'));
              container.appendChild(detailField('未读消息', room.unread + ' 条'));
              container.appendChild(detailField('创建者', room.creator));
              container.appendChild(detailField('创建时间', room.created));
              // --- 场景配置 ---
              var sceneConfig = el('div', { class: 'admin-room-scene-config' });
              var sectionTitle = el('div', { class: 'admin-scene-title' }, '场景配置');
              sceneConfig.appendChild(sectionTitle);
              var sceneParts = [];
              var il = room.image_layer;
              var hl = room.hotspot_layer;
              if (il && il.preset) {
                sceneParts.push('预设: ' + il.preset);
                if (il.layer_id) sceneParts.push('图层: ' + il.layer_id);
              }
              if (hl && hl.hotspots && hl.hotspots.length) {
                sceneParts.push(hl.hotspots.length + ' 个热点');
              }
              var sceneText = sceneParts.length ? sceneParts.join(' · ') : '未自定义场景（使用默认）';
              sceneConfig.appendChild(detailField('当前场景', sceneText));
              var presetSelect = el('select', { class: 'ds-select admin-scene-preset' });
              var presetOpts = [
                { v: '', t: '默认（无自定义）' },
                { v: 'creative-room', t: '创意房间 · creative-room' },
                { v: 'main-city', t: '主城夜景 · main-city' },
                { v: 'contract-private-room', t: '合约私室 · contract-private-room' },
                { v: 'contract-square-night', t: '合约广场 · contract-square-night' }
              ];
              for (var oi = 0; oi < presetOpts.length; oi++) {
                var opt = el('option', { value: presetOpts[oi].v }, presetOpts[oi].t);
                if (il && il.preset === presetOpts[oi].v) opt.selected = true;
                presetSelect.appendChild(opt);
              }
              sceneConfig.appendChild(detailField('选择预设', presetSelect));

              // --- 自定义背景图（白天+夜晚必须成对）---
              var dayUrlInput = el('input', {
                type: 'text', class: 'ds-input', placeholder: '白天背景图 URL（可选）',
                value: (il && il.day_image_url) ? il.day_image_url : '',
                style: 'width:100%;margin-top:6px;'
              });
              var nightUrlInput = el('input', {
                type: 'text', class: 'ds-input', placeholder: '夜晚背景图 URL（可选）',
                value: (il && il.night_image_url) ? il.night_image_url : '',
                style: 'width:100%;margin-top:4px;'
              });
              sceneConfig.appendChild(el('div', { style: 'margin-top:8px;' }, [
                el('label', { style: 'font-size:11px;color:var(--ds-text-secondary);' }, '自定义背景（白天+夜晚必须成对填写）'),
                dayUrlInput,
                nightUrlInput
              ]));

              // --- 热点编辑器（共享 createHotspotListEditor，2026-08-02 去重） ---
              var hotspotSection = el('div', { class: 'admin-hotspot-editor' });
              var hotspotTitle = el('div', { class: 'admin-scene-subtitle' }, '热点配置');
              hotspotSection.appendChild(hotspotTitle);

              var existingHotspots = (hl && hl.hotspots && hl.hotspots.length) ? hl.hotspots : [];
              var hotspotEditor = createHotspotListEditor(existingHotspots, { layout: 'blocks' });
              hotspotSection.appendChild(hotspotEditor.listEl);

              var addHotspotBtn = makeBtn('+ 添加热点', 'ds-btn-outline ds-btn-xs');
              addHotspotBtn.addEventListener('click', hotspotEditor.addHotspot);
              hotspotSection.appendChild(el('div', { style: 'margin-top:6px;' }, addHotspotBtn));
              sceneConfig.appendChild(hotspotSection);

              var applyBtn = makeBtn('应用场景', 'ds-btn-primary ds-btn-sm');
              var statusMsg = el('span', { class: 'admin-scene-msg' });
              applyBtn.addEventListener('click', async function () {
                applyBtn.disabled = true; applyBtn.textContent = '保存中...';
                statusMsg.textContent = ''; statusMsg.style.color = '';
                var selectedPreset = presetSelect.value;

                var hlPayload = existingHotspots.length
                  ? buildHotspotLayerPayload(hotspotEditor.collectHotspots())
                  : null;

                try {
                  var dayUrl = dayUrlInput.value.trim();
                  var nightUrl = nightUrlInput.value.trim();
                  var ilPayload = buildImageLayerPayload(selectedPreset, dayUrl, nightUrl, 'admin-custom-');
                  var res = await fetchGatewayJsonPost('/v1/admin/scene', {
                    room_id: room.id,
                    image_layer: ilPayload,
                    hotspot_layer: hlPayload
                  });
                  if (res.error) { statusMsg.textContent = '失败: ' + res.error; statusMsg.style.color = 'var(--ds-danger)'; }
                  else if (!res.ok) { statusMsg.textContent = '失败 (HTTP ' + res.status + ')'; statusMsg.style.color = 'var(--ds-danger)'; }
                  else {
                    statusMsg.textContent = '场景已应用'; statusMsg.style.color = 'var(--ds-success)';
                    renderRooms(document.getElementById('roomTypeFilter').value, document.getElementById('roomSearch').value);
                  }
                } catch (err) {
                  statusMsg.textContent = '请求异常: ' + (err.message || ''); statusMsg.style.color = 'var(--ds-danger)';
                }
                applyBtn.disabled = false; applyBtn.textContent = '应用场景';
              });
              sceneConfig.appendChild(el('div', { class: 'admin-scene-actions' }, applyBtn, statusMsg));
              container.appendChild(sceneConfig);
            },
            function (actions) {
              var viewMsgBtn = makeBtn('查看消息', 'ds-btn-outline ds-btn-sm');
              viewMsgBtn.addEventListener('click', function () {
                switchModule('messages');
                var searchInput = document.getElementById('msgSearch');
                if (searchInput) {
                  searchInput.value = room.name;
                  renderMessages('all', 'all', room.name);
                }
                showAdminNotice('已跳转到消息审核，可继续按房间名检索', 'info');
              });
              actions.appendChild(viewMsgBtn);
              var memberBtn = makeBtn('管理成员', 'ds-btn-outline ds-btn-sm');
              memberBtn.addEventListener('click', function () {
                if (!room) return;
                var residentId = prompt('输入居民ID（添加/移除）:');
                if (!residentId) return;
                var action = confirm('确定要切换该居民在本房间的成员状态？\n按确定=添加, 取消=移除') ? 'add' : 'remove';
                memberBtn.disabled = true; memberBtn.textContent = '处理中...';
                fetchGatewayJsonPost('/v1/admin/rooms/members', {room_id: room.id, resident_id: residentId, actor_id: currentGatewayIdentity(), action: action}).then(function(r) {
                  memberBtn.disabled = false; memberBtn.textContent = '管理成员';
                  if (r.error) { showAdminNotice('成员操作失败: ' + r.error, 'error'); }
                  else if (r.ok) { showAdminNotice('成员 ' + residentId + ' 已' + (action==='add'?'添加至':'移出') + '房间 ' + room.id, 'success'); }
                  else { showAdminNotice('成员操作失败 (HTTP ' + r.status + ')', 'error'); }
                });
              });
              actions.appendChild(memberBtn);

              if (room.frozen) {
                var unfreezeBtn = makeBtn('解冻房间', 'ds-btn-outline ds-btn-sm');
                unfreezeBtn.style.color = 'var(--ds-success)';
                unfreezeBtn.addEventListener('click', function () { closeDetail(); unfreezeRoom(room.id, unfreezeBtn); });
                actions.appendChild(unfreezeBtn);
              } else {
                var freezeBtn = makeBtn('冻结房间', 'ds-btn-outline ds-btn-sm');
                freezeBtn.style.color = 'var(--ds-danger)';
                freezeBtn.addEventListener('click', function () { closeDetail(); freezeRoom(room.id, freezeBtn); });
                actions.appendChild(freezeBtn);
              }
            }
          );
        });

        tbody.appendChild(tr);
      })(roomPage[i]);
    }

    renderPagination('rooms', filtered.length, function(p) { renderRooms(filterType, searchTerm); });
  }

  document.getElementById('roomSearch').addEventListener('input', function () {
    renderRooms(document.getElementById('roomTypeFilter').value, this.value);
  });
  document.getElementById('roomTypeFilter').addEventListener('change', function () {
    renderRooms(this.value, document.getElementById('roomSearch').value);
  });

  // ====== Render Messages Table ======

  function buildContextMessages(container, currentMsg) {
    var ctxBox = el('div', {
      style: 'background:var(--ds-bg);padding:10px;border-radius:var(--ds-radius);font-size:12px;color:var(--ds-text-secondary);'
    });

    // 从真实 messages 数组提取相邻上下文
    var contextMessages = [];
    var currentIdx = -1;
    for (var i = 0; i < messages.length; i++) {
      if (messages[i].sender === currentMsg.sender && messages[i].content === currentMsg.content && messages[i].room === currentMsg.room) {
        currentIdx = i;
        break;
      }
    }

    if (currentIdx >= 0) {
      // 取同 room 的前 2 条和后 2 条
      var sameRoomMsgs = [];
      for (var j = 0; j < messages.length; j++) {
        if (messages[j].room === currentMsg.room) sameRoomMsgs.push(j);
      }
      var roomPos = sameRoomMsgs.indexOf(currentIdx);
      if (roomPos >= 0) {
        var start = Math.max(0, roomPos - 2);
        var end = Math.min(sameRoomMsgs.length, roomPos + 3);
        for (var k = start; k < end; k++) {
          if (sameRoomMsgs[k] !== currentIdx) {
            contextMessages.push(messages[sameRoomMsgs[k]]);
          }
        }
      }
    }

    if (!contextMessages.length) {
      var emptyNote = el('div', { style: 'text-align:center;padding:8px;color:var(--ds-text-secondary);' });
      emptyNote.textContent = '暂无上下文消息';
      ctxBox.appendChild(emptyNote);
    } else {
      for (var c = 0; c < contextMessages.length; c++) {
        var ctx = contextMessages[c];
        var line = el('div', { style: 'margin-bottom:4px;' });
        var timeStr = ctx.time || '';
        if (timeStr) {
          line.appendChild(el('strong', null, timeStr + ' '));
        }
        line.appendChild(document.createTextNode(ctx.sender + ': ' + ctx.content));
        ctxBox.appendChild(line);
      }
    }

    // 当前消息高亮行
    var currentLine = el('div', { style: 'margin-bottom:4px;color:var(--ds-text);font-weight:bold;' });
    var currentTime = currentMsg.time || '';
    if (currentTime) {
      currentLine.appendChild(el('strong', null, currentTime + ' '));
    }
    currentLine.appendChild(document.createTextNode(currentMsg.sender + ': ' + currentMsg.content));
    ctxBox.appendChild(currentLine);

    return ctxBox;
  }

  function renderMessages(filterRoom, filterStatus, searchTerm) {
    var tbody = document.getElementById('msgTableBody');
    var filtered = messages.filter(function (m) {
      if (filterRoom && filterRoom !== 'all') {
        var roomMap = { main: '#主城大厅', villa: '#望海别墅', market: '#世界广场' };
        if (m.room !== roomMap[filterRoom]) return false;
      }
      if (filterStatus && filterStatus !== 'all' && m.status !== filterStatus) return false;
      if (searchTerm) {
        if (m.content.toLowerCase().indexOf(searchTerm.toLowerCase()) === -1 &&
            m.sender.toLowerCase().indexOf(searchTerm.toLowerCase()) === -1 &&
            m.room.toLowerCase().indexOf(searchTerm.toLowerCase()) === -1) return false;
      }
      return true;
    });

    updateAlertCounts();

    clear(tbody);

    if (!filtered.length) { renderEmptyRow(tbody, 7, '暂无消息数据'); renderPagination('messages', 0, function(p){ renderMessages(filterRoom, filterStatus, searchTerm); }); return; }

    var msgPage = paginateArray(filtered, pageState.messages || 1);
    for (var i = 0; i < msgPage.length; i++) {
      (function (msg) {
        var tr = el('tr', { data: { msgSender: msg.sender } });

        // 时间
        var tdTime = el('td');
        tdTime.style.cssText = 'font-family:var(--ds-font-mono);font-size:12px;color:var(--ds-text-secondary);';
        tdTime.textContent = msg.time;
        tr.appendChild(tdTime);

        // 发送者
        var tdSender = el('td');
        tdSender.appendChild(el('strong', null, msg.sender));
        tr.appendChild(tdSender);

        // 房间
        tr.appendChild(makeTd(msg.room));

        // 消息内容
        var tdContent = el('td');
        tdContent.style.cssText = 'max-width:280px;overflow:hidden;text-overflow:ellipsis;';
        tdContent.textContent = msg.content;
        tr.appendChild(tdContent);

        // 状态标签
        var tdStatus = el('td');
        tdStatus.appendChild(makeTag(L.msgStatusText[msg.status] || msg.status, L.msgStatusTag[msg.status] || 'default'));
        tr.appendChild(tdStatus);

        // 操作按钮
        var tdActions = el('td');
        var btnGroup = makeBtnGroup();

        var ctxBtn = makeBtn('上下文', 'ds-btn-outline ds-btn-xs');
        ctxBtn.addEventListener('click', function (e) {
          e.stopPropagation();
          openDetail(
            '消息上下文: ' + msg.sender,
            function (container) {
              container.appendChild(detailField('房间', msg.room));
              var ctxLabel = el('div', { class: 'ds-detail-label' }, '上下文消息');
              var field2 = el('div', { class: 'ds-detail-field' });
              field2.appendChild(ctxLabel);
              field2.appendChild(buildContextMessages(container, msg));
              container.appendChild(field2);
            },
            function (actions) {
              var handleBtn = makeBtn('标记已处理', 'ds-btn-primary ds-btn-sm');
              handleBtn.addEventListener('click', function (e2) {
                e2.stopPropagation();
                if (handleBtn.disabled) return;
                handleBtn.disabled = true;
                handleBtn.textContent = '...';
                moderateMessage(msg.message_id, msg.conversation_id, 'handled').then(function (result) {
                  if (result.error) {
                    showAdminNotice('操作失败: ' + result.error, 'error');
                    handleBtn.disabled = false;
                    handleBtn.textContent = '标记已处理';
                    return;
                  }
                  if (!result.ok) {
                    var errMsg2 = (result.data && result.data.error) || '请求失败';
                    showAdminNotice('操作失败: ' + errMsg2, 'error');
                    handleBtn.disabled = false;
                    handleBtn.textContent = '标记已处理';
                    return;
                  }
                  msg.status = 'handled';
                  showAdminNotice('消息已标记为已处理', 'success');
                  refreshCurrentMessageView();
                });
              });
              actions.appendChild(handleBtn);
            }
          );
        });
        btnGroup.appendChild(ctxBtn);

        if (msg.status === 'pending' || msg.status === 'flagged') {
          var passBtn = makeBtn('通过', 'ds-btn-primary ds-btn-xs');
          passBtn.addEventListener('click', function (e) {
            e.stopPropagation();
            if (passBtn.disabled) return;
            passBtn.disabled = true;
            passBtn.textContent = '...';
            moderateMessage(msg.message_id, msg.conversation_id, 'approved').then(function (result) {
              if (result.error) {
                showAdminNotice('审核失败: ' + result.error, 'error');
                passBtn.disabled = false;
                passBtn.textContent = '通过';
                return;
              }
              if (!result.ok) {
                var errMsg = (result.data && result.data.error) || '请求失败';
                showAdminNotice('审核失败: ' + errMsg, 'error');
                passBtn.disabled = false;
                passBtn.textContent = '通过';
                return;
              }
              msg.status = 'approved';
              showAdminNotice('消息已通过', 'success');
              refreshCurrentMessageView();
            });
          });
          btnGroup.appendChild(passBtn);

          var blockBtn = makeBtn('屏蔽', 'ds-btn-danger-text ds-btn-xs');
          blockBtn.addEventListener('click', function (e) {
            e.stopPropagation();
            if (blockBtn.disabled) return;
            blockBtn.disabled = true;
            blockBtn.textContent = '...';
            moderateMessage(msg.message_id, msg.conversation_id, 'blocked').then(function (result) {
              if (result.error) {
                showAdminNotice('操作失败: ' + result.error, 'error');
                blockBtn.disabled = false;
                blockBtn.textContent = '屏蔽';
                return;
              }
              if (!result.ok) {
                var errMsg = (result.data && result.data.error) || '请求失败';
                showAdminNotice('操作失败: ' + errMsg, 'error');
                blockBtn.disabled = false;
                blockBtn.textContent = '屏蔽';
                return;
              }
              msg.status = 'blocked';
              showAdminNotice('消息已屏蔽', 'success');
              refreshCurrentMessageView();
            });
          });
          btnGroup.appendChild(blockBtn);
        }

        tdActions.appendChild(btnGroup);
        tr.appendChild(tdActions);

        // 行点击 → 详情
        tr.addEventListener('click', function (e) {
          if (e.target.closest('button')) return;
          var prev = tbody.querySelectorAll('tr.selected');
          for (var p = 0; p < prev.length; p++) { prev[p].classList.remove('selected'); }
          tr.classList.add('selected');

          var mst = L.msgStatusText[msg.status] || msg.status;
          var mstag = L.msgStatusTag[msg.status] || 'default';

          openDetail(
            '消息详情',
            function (container) {
              container.appendChild(detailField('时间', msg.time));
              container.appendChild(detailField('发送者', msg.sender));
              container.appendChild(detailField('房间', msg.room));
              container.appendChild(detailField('消息内容', msg.content));
              container.appendChild(detailField('审核状态', makeTag(mst, mstag)));
              var ctxField = el('div', { class: 'ds-detail-field' });
              ctxField.appendChild(el('div', { class: 'ds-detail-label' }, '上下文消息'));
              ctxField.appendChild(buildContextMessages(container, msg));
              container.appendChild(ctxField);
            },
            function (actions) {
              var copyMsgBtn = makeBtn('复制消息ID', 'ds-btn-outline ds-btn-sm');
              copyMsgBtn.addEventListener('click', function () {
                copyText([msg.time, msg.sender, msg.room].join(' | '), '已复制消息定位信息');
              });
              actions.appendChild(copyMsgBtn);
              if (msg.status === 'pending' || msg.status === 'flagged') {
                var handleBtn = makeBtn('标记已处理', 'ds-btn-primary ds-btn-sm');
              handleBtn.addEventListener('click', function (e2) {
                e2.stopPropagation();
                if (handleBtn.disabled) return;
                handleBtn.disabled = true;
                handleBtn.textContent = '...';
                moderateMessage(msg.message_id, msg.conversation_id, 'handled').then(function (result) {
                  if (result.error) {
                    showAdminNotice('操作失败: ' + result.error, 'error');
                    handleBtn.disabled = false;
                    handleBtn.textContent = '标记已处理';
                    return;
                  }
                  if (!result.ok) {
                    var errMsg2 = (result.data && result.data.error) || '请求失败';
                    showAdminNotice('操作失败: ' + errMsg2, 'error');
                    handleBtn.disabled = false;
                    handleBtn.textContent = '标记已处理';
                    return;
                  }
                  msg.status = 'handled';
                  showAdminNotice('消息已标记为已处理', 'success');
                  refreshCurrentMessageView();
                });
              });
              actions.appendChild(handleBtn);
              }
            }
          );
        });

        tbody.appendChild(tr);
      })(msgPage[i]);
    }

    renderPagination('messages', filtered.length, function(p) { renderMessages(filterRoom, filterStatus, searchTerm); });
  }

  document.getElementById('msgSearch').addEventListener('input', function () {
    renderMessages(
      document.getElementById('msgRoomFilter').value,
      document.getElementById('msgStatusFilter').value,
      this.value
    );
  });
  document.getElementById('msgRoomFilter').addEventListener('change', function () {
    renderMessages(this.value, document.getElementById('msgStatusFilter').value, document.getElementById('msgSearch').value);
  });
  document.getElementById('msgStatusFilter').addEventListener('change', function () {
    renderMessages(document.getElementById('msgRoomFilter').value, this.value, document.getElementById('msgSearch').value);
  });

  // ====== Render Invite Codes ======

  function renderInvites() {
    var tbody = document.getElementById('inviteTableBody');
    clear(tbody);

    if (!inviteCodes.length) { renderEmptyRow(tbody, 6, '暂无邀请码数据'); renderPagination('permissions', 0, function(p){ renderInvites(); }); return; }

    var invitePage = paginateArray(inviteCodes, pageState.permissions || 1);
    for (var i = 0; i < invitePage.length; i++) {
      (function (ic) {
        var tr = el('tr');

        tr.appendChild(makeTdMono(ic.code));
        tr.appendChild(makeTd(ic.room));
        tr.appendChild(makeTd(String(ic.maxUses)));
        tr.appendChild(makeTd(String(ic.used)));
        tr.appendChild(makeTd(ic.expires));
        tr.appendChild(makeTd(ic.creator));

        var tdStatus = el('td');
        tdStatus.appendChild(makeTag(L.inviteStatusText[ic.status] || ic.status, L.inviteStatusTag[ic.status] || 'default'));
        tr.appendChild(tdStatus);

        var tdActions = el('td');
        var btnGroup = makeBtnGroup();
        var copyBtn = makeBtn('复制', 'ds-btn-outline ds-btn-xs');
        copyBtn.addEventListener('click', function (e) {
          e.stopPropagation();
          copyText(ic.code, '已复制邀请码');
        });
        btnGroup.appendChild(copyBtn);
        if (ic.status === 'active') {
          var revokeBtn = makeBtn('作废', 'ds-btn-danger-text ds-btn-xs');
          revokeBtn.addEventListener('click', function () {
                if (!confirm('确定要作废邀请码 ' + ic.code + ' ?')) return;
                revokeBtn.disabled = true; revokeBtn.textContent = '作废中...';
                fetchGatewayJsonPost('/v1/admin/invites/revoke', {code: ic.code, actor_id: currentGatewayIdentity()}).then(function(r) {
                  revokeBtn.disabled = false; revokeBtn.textContent = '已作废';
                  if (r.error) { showAdminNotice('作废失败: ' + r.error, 'error'); revokeBtn.textContent = '作废'; }
                  else if (r.ok) { showAdminNotice('邀请码 ' + ic.code + ' 已作废', 'success'); revokeBtn.textContent = '已作废'; revokeBtn.style.color = 'var(--ds-text-muted)'; }
                  else { showAdminNotice('作废失败 (HTTP ' + r.status + ')', 'error'); revokeBtn.textContent = '作废'; }
                });
              });
          btnGroup.appendChild(revokeBtn);
        }
        tdActions.appendChild(btnGroup);
        tr.appendChild(tdActions);

        tbody.appendChild(tr);
      })(invitePage[i]);
    }

    renderPagination('permissions', inviteCodes.length, function(p) { renderInvites(); });
  }

  // ====== Audit Log ======

  function formatAuditTime(ms) {
    var d = new Date(ms);
    var h = d.getHours().toString().padStart(2, '0');
    var m = d.getMinutes().toString().padStart(2, '0');
    var s = d.getSeconds().toString().padStart(2, '0');
    return h + ':' + m + ':' + s;
  }

  function auditEventToLog(event) {
    var action = event.action || '';
    var level = 'info';
    var type = 'audit_config';
    if (action === 'admin:ban_resident') { level = 'warn'; type = 'audit_security'; }
    else if (action === 'admin:unban_resident') { level = 'info'; type = 'audit_security'; }
    else if (action === 'admin:freeze_room') { level = 'warn'; type = 'audit_security'; }
    else if (action === 'admin:unfreeze_room') { level = 'info'; type = 'audit_security'; }
    else if (action.indexOf('admin:moderate_message') === 0) { level = 'info'; type = 'audit_content'; }
    else if (action === 'admin:create_permission_group') { level = 'info'; type = 'audit_permission'; }
    else if (action === 'admin:assign_permission_group') { level = 'info'; type = 'audit_permission'; }
    var desc = action + ' → ' + (event.target || '');
    if (event.reason) { desc = desc + ' (' + event.reason + ')'; }
    return {
      id: event.event_id || '',
      time: formatAuditTime(event.timestamp_ms),
      level: level,
      type: type,
      desc: desc,
      source: event.actor_id || ''
    };
  }

  async function loadAuditLog() {
    if (!gatewayUrl) {
      showAdminNotice('Gateway 未连接，无法加载审计日志', 'error');
      renderLogs('all', 'all', '');
      return;
    }
    try {
      var result = await fetchGatewayJson('/v1/admin/audit-log?limit=200');
      if (result && Array.isArray(result.events)) {
        auditEvents = result.events;
        gatewayAuditLogs = [];
        for (var i = 0; i < auditEvents.length; i++) {
          gatewayAuditLogs.push(auditEventToLog(auditEvents[i]));
        }
        // Gateway 返回空数组也是正式空态，不能回退到本地 mock。
        logs = gatewayAuditLogs;
      } else {
        auditEvents = [];
        gatewayAuditLogs = [];
        logs = [];
        showAdminNotice('Gateway 审计日志读取失败，已显示空态', 'error');
      }
    } catch (e) {
      showAdminNotice('加载审计日志失败: ' + (e.message || '网络错误'), 'error');
      auditEvents = [];
      gatewayAuditLogs = [];
      logs = [];
    }
    renderLogs(
      document.getElementById('logLevelFilter') ? document.getElementById('logLevelFilter').value : 'all',
      document.getElementById('logTypeFilter') ? document.getElementById('logTypeFilter').value : 'all',
      document.getElementById('logSearch') ? document.getElementById('logSearch').value : ''
    );
  }

  // ====== Render Logs ======

  function renderLogs(filterLevel, filterType, searchTerm) {
    updateAlertCounts();
    var tbody = document.getElementById('logTableBody');
    var filtered = logs.filter(function (l) {
      if (filterLevel && filterLevel !== 'all' && l.level !== filterLevel) return false;
      if (filterType && filterType !== 'all' && l.type !== filterType) return false;
      if (searchTerm) {
        if (l.desc.toLowerCase().indexOf(searchTerm.toLowerCase()) === -1 &&
            l.source.toLowerCase().indexOf(searchTerm.toLowerCase()) === -1) return false;
      }
      return true;
    });

    clear(tbody);

    if (!filtered.length) { renderEmptyRow(tbody, 6, '暂无日志数据'); renderPagination('logs', 0, function(p){ renderLogs(filterLevel, filterType, searchTerm); }); return; }

    var logPage = paginateArray(filtered, pageState.logs || 1);
    for (var i = 0; i < logPage.length; i++) {
      (function (log, idx) {
        var tr = el('tr');

        var tdTime = el('td');
        tdTime.style.cssText = 'font-family:var(--ds-font-mono);font-size:12px;color:var(--ds-text-secondary);';
        tdTime.textContent = log.time;
        tr.appendChild(tdTime);

        var tdLevel = el('td');
        tdLevel.appendChild(makeLogLevel(L.logLevelText[log.level] || log.level, log.level));
        tr.appendChild(tdLevel);

        tr.appendChild(makeTd(L.logTypeText[log.type] || log.type));
        tr.appendChild(makeTd(log.desc));
        tr.appendChild(makeTd(log.source, 'color:var(--ds-text-secondary);'));

        // 行点击 → 详情
        tr.addEventListener('click', function () {
          var prev = tbody.querySelectorAll('tr.selected');
          for (var p = 0; p < prev.length; p++) { prev[p].classList.remove('selected'); }
          tr.classList.add('selected');

          var lvText = L.logLevelText[log.level] || log.level;
          var ltText = L.logTypeText[log.type] || log.type;

          openDetail(
            '日志详情',
            function (container) {
              container.appendChild(detailFieldStyled('时间', log.time, 'font-family:var(--ds-font-mono);'));
              container.appendChild(detailField('级别', makeLogLevel(lvText, log.level)));
              container.appendChild(detailField('类型', ltText));
              container.appendChild(detailField('描述', log.desc));
              container.appendChild(detailField('来源模块', log.source));
            },
            function (actions) {
              var handleLogBtn = makeBtn('标记已处理', 'ds-btn-outline ds-btn-sm');
              handleLogBtn.addEventListener('click', function () {
                handleLogBtn.disabled = true; handleLogBtn.textContent = '处理中...';
                fetchGatewayJsonPost('/v1/admin/logs/handle', {log_id: log.id, actor_id: currentGatewayIdentity()}).then(function(r) {
                  handleLogBtn.disabled = false;
                  if (r.error) { showAdminNotice('标记失败: ' + r.error, 'error'); handleLogBtn.textContent = '标记已处理'; }
                  else if (r.ok) { showAdminNotice('日志 ' + log.id + ' 已标记为已处理', 'success'); handleLogBtn.textContent = '已处理'; handleLogBtn.style.color = 'var(--ds-success)'; }
                  else { showAdminNotice('标记失败 (HTTP ' + r.status + ')', 'error'); handleLogBtn.textContent = '标记已处理'; }
                });
              });
              actions.appendChild(handleLogBtn);
              var relatedBtn = makeBtn('查看相关日志', 'ds-btn-outline ds-btn-sm');
              relatedBtn.addEventListener('click', function () {
                var typeFilter = document.getElementById('logTypeFilter');
                var searchInput = document.getElementById('logSearch');
                if (typeFilter) typeFilter.value = log.type;
                if (searchInput) searchInput.value = log.source;
                renderLogs('all', log.type, log.source);
                showAdminNotice('已筛选同类来源日志', 'info');
              });
              actions.appendChild(relatedBtn);
            }
          );
        });

        tbody.appendChild(tr);
      })(logPage[i], i);
    }

    renderPagination('logs', filtered.length, function(p) { renderLogs(filterLevel, filterType, searchTerm); });
  }

  document.getElementById('logSearch').addEventListener('input', function () {
    renderLogs(
      document.getElementById('logLevelFilter').value,
      document.getElementById('logTypeFilter').value,
      this.value
    );
  });
  document.getElementById('logLevelFilter').addEventListener('change', function () {
    renderLogs(this.value, document.getElementById('logTypeFilter').value, document.getElementById('logSearch').value);
  });
  document.getElementById('logTypeFilter').addEventListener('change', function () {
    renderLogs(document.getElementById('logLevelFilter').value, this.value, document.getElementById('logSearch').value);
  });

  // ====== Scene Editor Module ======

  function loadSceneModule() {
    var sel = document.getElementById('sceneRoomSelect');
    if (!sel) return;
    // Populate room selector from current rooms data
    while (sel.firstChild) sel.removeChild(sel.firstChild);
    var defaultOpt = document.createElement('option');
    defaultOpt.value = '';
    defaultOpt.textContent = '-- 请选择房间 --';
    sel.appendChild(defaultOpt);
    for (var i = 0; i < rooms.length; i++) {
      var opt = document.createElement('option');
      opt.value = rooms[i].id;
      opt.textContent = rooms[i].name + ' (' + rooms[i].id + ')';
      sel.appendChild(opt);
    }
    if (!sel._listenerBound) {
      sel._listenerBound = true;
      sel.addEventListener('change', function () {
        var roomId = this.value;
        var container = document.getElementById('sceneEditorContainer');
        if (!roomId) {
          clear(container);
          var placeholderP = document.createElement('p');
          placeholderP.style.cssText = 'color:var(--ds-text-muted);font-size:13px;';
          placeholderP.textContent = '请先选择一个房间以编辑其场景配置。';
          container.appendChild(placeholderP);
          return;
        }
        var room = null;
        for (var r = 0; r < rooms.length; r++) {
          if (rooms[r].id === roomId) { room = rooms[r]; break; }
        }
        if (room) renderSceneEditor(room, container);
      });
    }
    // Auto-select if rooms data available
    if (rooms.length && !sel.value) {
      sel.value = rooms[0].id;
      sel.dispatchEvent(new Event('change'));
    }
  }

  function renderSceneEditor(room, container) {
    clear(container);
    var il = room.image_layer;
    var hl = room.hotspot_layer;

    // 可视化编辑器入口：scene-editor.html 与 admin-ds.html 同目录，URL 合同与
    // app.js sceneEditorUrlForCurrentState() 一致（gateway/room/token/identity）。
    if (gatewayUrl && room && room.id) {
      var visualEditorUrl = './scene-editor.html?gateway=' + encodeURIComponent(gatewayUrl) +
        '&room=' + encodeURIComponent(room.id);
      var editorSessionToken = safeLocalStorageGet('lobster-session-token');
      if (editorSessionToken) {
        visualEditorUrl += '&token=' + encodeURIComponent(editorSessionToken) +
          '&identity=' + encodeURIComponent(currentGatewayIdentity());
      }
      var entryBar = el('div', { class: 'ds-card', style: 'margin-bottom:1rem;padding:0.75rem 1rem;display:flex;align-items:center;gap:0.75rem;flex-wrap:wrap;' });
      var entryLink = el('a', {
        class: 'ds-btn ds-btn-primary ds-btn-sm',
        href: visualEditorUrl,
        target: '_blank',
        rel: 'noopener'
      }, '打开可视化编辑器（拖拽/缩放热点）');
      entryBar.appendChild(entryLink);
      entryBar.appendChild(el('span', { style: 'font-size:12px;color:var(--ds-text-secondary);' }, '在 16:9 画布上可视化编辑背景与热点，新标签页打开'));
      container.appendChild(entryBar);
    }

    // Image layer section
    var imgSection = el('div', { class: 'ds-card', style: 'margin-bottom:1rem;' });
    imgSection.appendChild(el('div', { class: 'ds-card-header' },
      el('span', { class: 'ds-card-title' }, '图像层配置')
    ));

    var presetSelect = el('select', { class: 'ds-select', style: 'width:100%;max-width:320px;' });
    var presetOpts = [
      { v: '', t: '默认（无自定义）' },
      { v: 'creative-room', t: '创意房间 · creative-room' },
      { v: 'main-city', t: '主城夜景 · main-city' },
      { v: 'contract-private-room', t: '合约私室 · contract-private-room' },
      { v: 'contract-square-night', t: '合约广场 · contract-square-night' }
    ];
    for (var oi = 0; oi < presetOpts.length; oi++) {
      var opt = el('option', { value: presetOpts[oi].v }, presetOpts[oi].t);
      if (il && il.preset === presetOpts[oi].v) opt.selected = true;
      presetSelect.appendChild(opt);
    }
    imgSection.appendChild(el('div', { style: 'padding:0.75rem 1rem;' },
      el('label', { style: 'display:block;margin-bottom:4px;font-size:12px;color:var(--ds-text-secondary);' }, '场景预设'),
      presetSelect
    ));
    var dayUrlInput = el('input', {
      type: 'text',
      class: 'ds-input',
      placeholder: '白天背景图 URL（可选）',
      value: (il && il.day_image_url) ? il.day_image_url : '',
      style: 'width:100%;max-width:520px;margin-top:8px;'
    });
    var nightUrlInput = el('input', {
      type: 'text',
      class: 'ds-input',
      placeholder: '夜晚背景图 URL（可选）',
      value: (il && il.night_image_url) ? il.night_image_url : '',
      style: 'width:100%;max-width:520px;margin-top:6px;'
    });
    imgSection.appendChild(el('div', { style: 'padding:0 1rem 0.75rem;' },
      el('label', { style: 'display:block;margin-bottom:4px;font-size:12px;color:var(--ds-text-secondary);' }, '自定义背景（白天+夜晚必须成对填写）'),
      dayUrlInput,
      nightUrlInput
    ));
    if (il && il.layer_id) {
      imgSection.appendChild(el('div', { style: 'padding:0 1rem 0.75rem;font-size:11px;color:var(--ds-text-muted);' }, '图层ID: ' + il.layer_id));
    }
    container.appendChild(imgSection);

    // Hotspot layer section（共享 createHotspotListEditor，2026-08-02 去重）
    var hsSection = el('div', { class: 'ds-card', style: 'margin-bottom:1rem;' });
    var hotspotTitle = el('span', { class: 'ds-card-title' }, '热点配置 (' + ((hl && hl.hotspots && hl.hotspots.length) || 0) + ' 个)');
    hsSection.appendChild(el('div', { class: 'ds-card-header' },
      hotspotTitle
    ));

    var existingHotspots = (hl && hl.hotspots && hl.hotspots.length) ? hl.hotspots.slice() : [];
    var hotspotEditor = createHotspotListEditor(existingHotspots, {
      layout: 'flex',
      onRowsRendered: function () {
        hotspotTitle.textContent = '热点配置 (' + existingHotspots.length + ' 个)';
      }
    });
    hsSection.appendChild(hotspotEditor.listEl);

    var addBtn = makeBtn('+ 添加热点', 'ds-btn-outline ds-btn-xs');
    addBtn.style.margin = '0.5rem 1rem';
    addBtn.addEventListener('click', hotspotEditor.addHotspot);
    hsSection.appendChild(el('div', { style: 'margin-top:6px;' }, addBtn));
    container.appendChild(hsSection);

    // Save button
    var actionsRow = el('div', { style: 'display:flex;align-items:center;gap:0.75rem;' });
    var saveBtn = makeBtn('保存场景', 'ds-btn-primary ds-btn-sm');
    var statusMsg = el('span', { style: 'font-size:12px;' });
    saveBtn.addEventListener('click', async function () {
      saveBtn.disabled = true; saveBtn.textContent = '保存中...';
      statusMsg.textContent = ''; statusMsg.style.color = '';
      var selectedPreset = presetSelect.value;

      var hlPayload = buildHotspotLayerPayload(hotspotEditor.collectHotspots());

      try {
        var dayUrl = dayUrlInput.value.trim();
        var nightUrl = nightUrlInput.value.trim();
        var ilPayload = buildImageLayerPayload(selectedPreset, dayUrl, nightUrl, 'admin-scene-');
        var res = await fetchGatewayJsonPost('/v1/admin/scene', {
          room_id: room.id,
          image_layer: ilPayload,
          hotspot_layer: hlPayload
        });
        if (res.error) { statusMsg.textContent = '失败: ' + res.error; statusMsg.style.color = 'var(--ds-danger)'; }
        else if (res.ok) {
          statusMsg.textContent = '场景已保存'; statusMsg.style.color = 'var(--ds-success)';
          // Refresh room data in-memory
          if (res.image_layer) room.image_layer = res.image_layer;
          if (res.hotspot_layer) room.hotspot_layer = res.hotspot_layer;
        } else { statusMsg.textContent = '保存失败'; statusMsg.style.color = 'var(--ds-danger)'; }
      } catch (err) {
        statusMsg.textContent = '请求异常: ' + (err.message || ''); statusMsg.style.color = 'var(--ds-danger)';
      }
      saveBtn.disabled = false; saveBtn.textContent = '保存场景';
    });
    actionsRow.appendChild(saveBtn);
    actionsRow.appendChild(statusMsg);
    container.appendChild(actionsRow);
  }

  // ====== Device Management ======
  function renderDeviceEmptyRow(tbody, message) {
    clear(tbody);
    var tr = el('tr');
    var td = el('td', { colspan: '6', class: 'ds-empty' });
    td.textContent = message;
    tr.appendChild(td);
    tbody.appendChild(tr);
  }

  async function loadDevices() {
    var tbody = document.getElementById('deviceTableBody');
    var countEl = document.getElementById('deviceCount');
    if (!tbody) return;
    renderDeviceEmptyRow(tbody, '加载中...');

    try {
      var devices = await fetchGatewayJson('/v1/admin/devices');
      if (!Array.isArray(devices)) { renderDeviceEmptyRow(tbody, '暂无设备数据'); return; }
      if (countEl) countEl.textContent = '(' + devices.length + ' 台设备)';
      clear(tbody);

      for (var i = 0; i < devices.length; i++) {
        (function (d) {
          var tr = el('tr');
          tr.appendChild(el('td', {}, el('code', { style: 'font-family:var(--ds-font-mono);font-size:12px;' }, d.address)));
          tr.appendChild(el('td', {}, d.label));
          var statusCell = el('td');
          if (d.blocked) {
            statusCell.appendChild(el('span', { class: 'ds-badge', style: 'background:var(--ds-danger);color:#fff;' }, '已封禁'));
          } else {
            statusCell.appendChild(el('span', { class: 'ds-badge', style: 'background:var(--ds-success);color:#fff;' }, '正常'));
          }
          tr.appendChild(statusCell);
          tr.appendChild(el('td', {}, d.bound_resident_id || '-'));
          tr.appendChild(el('td', { style: 'font-size:12px;color:var(--ds-text-secondary);' }, new Date(d.added_at_ms).toLocaleString('zh-CN')));

          var actionsCell = el('td');
          var btnGroup = makeBtnGroup();
          if (d.blocked) {
            var unblockBtn = makeBtn('解封', 'ds-btn-outline ds-btn-xs');
            unblockBtn.addEventListener('click', function () {
              fetchGatewayJsonPost('/v1/admin/devices/unblock', { address: d.address }).then(function (r) {
                if (r && r.error) { showAdminNotice('解封失败: ' + r.error, 'error'); }
                else if (r && r.ok) { showAdminNotice('设备 ' + d.address + ' 已解封', 'success'); }
                else { showAdminNotice('解封失败 (HTTP ' + (r && r.status || '?') + ')', 'error'); }
                loadDevices();
              });
            });
            btnGroup.appendChild(unblockBtn);
          } else {
            var blockBtn = makeBtn('封禁', 'ds-btn-outline ds-btn-xs');
            blockBtn.style.color = 'var(--ds-danger)';
            blockBtn.addEventListener('click', function () {
              fetchGatewayJsonPost('/v1/admin/devices/block', { address: d.address }).then(function (r) {
                if (r && r.error) { showAdminNotice('封禁失败: ' + r.error, 'error'); }
                else if (r && r.ok) { showAdminNotice('设备 ' + d.address + ' 已封禁', 'success'); }
                else { showAdminNotice('封禁失败 (HTTP ' + (r && r.status || '?') + ')', 'error'); }
                loadDevices();
              });
            });
            btnGroup.appendChild(blockBtn);
          }
          var removeBtn = makeBtn('移除', 'ds-btn-danger-text ds-btn-xs');
          removeBtn.addEventListener('click', function () {
            if (confirm('确定移除设备 ' + d.address + ' ？')) {
              fetchGatewayJsonPost('/v1/admin/devices/remove', { address: d.address }).then(function (r) {
                if (r && r.error) { showAdminNotice('移除失败: ' + r.error, 'error'); }
                else if (r && r.ok) { showAdminNotice('设备 ' + d.address + ' 已移除', 'success'); }
                else { showAdminNotice('移除失败 (HTTP ' + (r && r.status || '?') + ')', 'error'); }
                loadDevices();
              });
            }
          });
          btnGroup.appendChild(removeBtn);
          actionsCell.appendChild(btnGroup);
          tr.appendChild(actionsCell);
          tbody.appendChild(tr);
        })(devices[i]);
      }
    } catch (e) {
      renderDeviceEmptyRow(tbody, '加载失败: ' + (e.message || '未知错误'));
    }
  }

  // Bind device add button
  var deviceAddBtn = document.getElementById('deviceAddBtn');
  if (deviceAddBtn) {
    deviceAddBtn.addEventListener('click', async function () {
      var addr = document.getElementById('deviceAddressInput');
      var label = document.getElementById('deviceLabelInput');
      if (!addr || !addr.value.trim()) { showAdminNotice('请输入 MAC 地址', 'error'); return; }
      deviceAddBtn.disabled = true; deviceAddBtn.textContent = '添加中...';
      try {
        var res = await fetchGatewayJsonPost('/v1/admin/devices/add', {
          address: addr.value.trim(),
          label: (label && label.value.trim()) ? label.value.trim() : '未命名设备'
        });
        if (res.error) { showAdminNotice('添加失败: ' + res.error, 'error'); }
        else if (res.ok) { showAdminNotice('设备已添加', 'success'); addr.value = ''; if (label) label.value = ''; loadDevices(); }
        else { showAdminNotice('添加失败 (HTTP ' + res.status + ')', 'error'); }
      } catch (e) {
        showAdminNotice('添加失败: ' + (e.message || '网络错误'), 'error');
      } finally {
        deviceAddBtn.disabled = false; deviceAddBtn.textContent = '添加设备';
      }
    });
  }

  function bindStaticAdminActions() {
    var residentExport = document.querySelector('[data-admin-action="export-residents"]');
    if (residentExport) {
      residentExport.addEventListener('click', function () {
        downloadCsv('ajw-residents.csv', [
          { label: '居民ID', key: 'id' },
          { label: '昵称', key: 'nick' },
          { label: '邮箱', key: 'email' },
          { label: '角色', get: function (row) { return L.roleText[row.role] || row.role; } },
          { label: '状态', get: function (row) { return L.statusText[row.status] || row.status; } },
          { label: '最近在线', key: 'lastSeen' },
          { label: '消息数', key: 'msgCount' }
        ], filteredResidents());
      });
    }

    var logExport = document.querySelector('[data-admin-action="export-logs"]');
    if (logExport) {
      logExport.addEventListener('click', function () {
        downloadCsv('ajw-admin-logs.csv', [
          { label: '时间', key: 'time' },
          { label: '级别', get: function (row) { return L.logLevelText[row.level] || row.level; } },
          { label: '类型', get: function (row) { return L.logTypeText[row.type] || row.type; } },
          { label: '描述', key: 'desc' },
          { label: '来源', key: 'source' }
        ], filteredLogs());
      });
    }

    var refreshMessages = document.querySelector('[data-admin-action="refresh-messages"]');
    if (refreshMessages) {
      refreshMessages.addEventListener('click', async function () {
        await loadGatewayAdminData();
        renderMessages(
          document.getElementById('msgRoomFilter').value,
          document.getElementById('msgStatusFilter').value,
          document.getElementById('msgSearch').value
        );
        showAdminNotice(gatewayUrl ? '已刷新 Gateway 消息视图' : '已刷新本地预览数据，当前未连接 Gateway', gatewayUrl ? 'success' : 'warning');
      });
    }

    var unavailableActions = [];
    for (var i = 0; i < unavailableActions.length; i++) {
      var button = document.querySelector('[data-admin-action="' + unavailableActions[i][0] + '"]');
      markUnavailableButton(button, unavailableActions[i][1]);
    }

    // sysconfig: refresh from gateway (real GET + POST)
    // Wire create-permission-group button
    var createPgBtn = document.querySelector('[data-admin-action="create-permission-group"]');
    if (createPgBtn) {
      createPgBtn.addEventListener('click', async function () {
        var capabilities = await loadCapabilities();
        var capHelpLines = ['可选 capability:'];
        for (var ci = 0; ci < capabilities.length; ci++) {
          capHelpLines.push(capabilities[ci].key + ' - ' + capabilities[ci].label);
        }
        capHelpLines.push('');
        capHelpLines.push('请输入 capability key，多个用逗号分隔 (如: freeze:room, moderate:message):');
        var name = prompt('权限组名称 (如: 协管员):');
        if (!name) return;
        var desc = prompt('描述:') || '';
        var capInput = prompt(capHelpLines.join('\n'));
        if (!capInput) return;
        var capList = capInput.split(',').map(function(s) { return s.trim(); }).filter(Boolean);

        createPgBtn.disabled = true; createPgBtn.textContent = '创建中...';
        try {
          var resp = await fetchGatewayJsonPost('/v1/admin/permission-groups', {
            actor_id: currentGatewayIdentity(), name: name, description: desc, capabilities: capList
          });
          if (resp.error) { showAdminNotice('创建失败: ' + resp.error, 'error'); }
          else if (resp.ok) { showAdminNotice('权限组 ' + name + ' 已创建', 'success'); await loadPermissionGroups(); }
          else { showAdminNotice('创建失败 (HTTP ' + resp.status + ')', 'error'); }
        } catch (e) { showAdminNotice('创建请求失败', 'error'); }
        createPgBtn.disabled = false; createPgBtn.textContent = '+ 新建权限组';
      });
    }

    // Wire generate-invite button
    var genInviteBtn = document.querySelector('[data-admin-action="generate-invite"]');
    if (genInviteBtn) {
      genInviteBtn.addEventListener('click', function () {
        genInviteBtn.disabled = true; genInviteBtn.textContent = '生成中...';
        fetchGatewayJsonPost('/v1/admin/invites', {actor_id: currentGatewayIdentity(), max_uses: 10}).then(function(r) {
          genInviteBtn.disabled = false; genInviteBtn.textContent = '+ 生成邀请码';
          if (r.error) { showAdminNotice('生成失败: ' + r.error, 'error'); }
          else if (r.ok) { showAdminNotice('邀请码已生成: ' + (r.data && r.data.code || ''), 'success'); loadInviteCodes(); }
          else { showAdminNotice('生成失败 (HTTP ' + r.status + ')', 'error'); }
        }).catch(function() {
          genInviteBtn.disabled = false; genInviteBtn.textContent = '+ 生成邀请码';
        });
      });
    }

    // Wire batch-approve button
    var batchApproveBtn = document.querySelector('[data-admin-action="batch-approve-messages"]');
    if (batchApproveBtn) {
      batchApproveBtn.addEventListener('click', function () {
        var rows = document.querySelectorAll('[data-message-id]');
        if (!rows.length) { showAdminNotice('没有可审核的消息', 'info'); return; }
        if (!confirm('确定要批量通过当前可见的 ' + rows.length + ' 条消息？')) return;
        batchApproveBtn.disabled = true; batchApproveBtn.textContent = '批量通过中...';
        var promises = [];
        rows.forEach(function (row) {
          var msgId = row.dataset.messageId;
          var convId = row.dataset.conversationId || '';
          if (msgId) promises.push(fetchGatewayJsonPost('/v1/admin/messages/moderate', {message_id: msgId, conversation_id: convId, action: 'approved'}));
        });
        Promise.all(promises).then(function (results) {
          batchApproveBtn.disabled = false; batchApproveBtn.textContent = '批量通过';
          var s = summarizeBatchResults(results);
          if (s.fail === 0) { showAdminNotice('已批量通过 ' + s.ok + ' 条消息', 'success'); }
          else if (s.ok === 0) { showAdminNotice('批量通过全部失败（' + s.fail + ' 条）', 'error'); }
          else { showAdminNotice('批量通过 ' + s.ok + ' 条成功，' + s.fail + ' 条失败', 'info'); }
          refreshCurrentMessageView();
        }).catch(function () {
          batchApproveBtn.disabled = false; batchApproveBtn.textContent = '批量通过';
          showAdminNotice('批量通过请求异常', 'error');
        });
      });
    }

    // Wire clear-processed-logs button
    // Wire create-resident button
    var createResidentBtn = document.querySelector('[data-admin-action="create-resident"]');
    if (createResidentBtn) {
      createResidentBtn.addEventListener('click', function () {
        var residentId = prompt('居民ID:');
        if (!residentId) return;
        var email = prompt('邮箱:');
        if (!email) return;
        createResidentBtn.disabled = true; createResidentBtn.textContent = '创建中...';
        fetchGatewayJsonPost('/v1/admin/residents', {resident_id: residentId, email: email}).then(function(r) {
          createResidentBtn.disabled = false; createResidentBtn.textContent = '+ 新建居民';
          if (r.error) { showAdminNotice('创建失败: ' + r.error, 'error'); }
          else if (r.ok) { showAdminNotice('居民 ' + residentId + ' 已创建', 'success'); loadGatewayAdminData(); }
          else { showAdminNotice('创建失败 (HTTP ' + r.status + ')', 'error'); }
        }).catch(function() {
          createResidentBtn.disabled = false; createResidentBtn.textContent = '+ 新建居民';
        });
      });
    }

    var clearLogsBtn = document.querySelector('[data-admin-action="clear-processed-logs"]');
    if (clearLogsBtn) {
      clearLogsBtn.addEventListener('click', function () {
        if (!confirm('确定要清空所有已处理的日志？')) return;
        clearLogsBtn.disabled = true; clearLogsBtn.textContent = '清空中...';
        fetchGatewayJsonPost('/v1/admin/logs/clear', {}).then(function (r) {
          clearLogsBtn.disabled = false; clearLogsBtn.textContent = '清空已处理';
          if (r.error) { showAdminNotice('清空失败: ' + r.error, 'error'); }
          else if (r.ok) { showAdminNotice('已清空 ' + (r.data && r.data.cleared || '') + ' 条已处理日志', 'success'); loadAuditLog(); }
          else { showAdminNotice('清空失败 (HTTP ' + r.status + ')', 'error'); }
        }).catch(function () {
          clearLogsBtn.disabled = false; clearLogsBtn.textContent = '清空已处理';
        });
      });
    }

    var refreshSysConfig = document.querySelector('[data-admin-action="refresh-sysconfig"]');
    if (refreshSysConfig) {
      refreshSysConfig.addEventListener('click', loadSysConfig);
    }

    var addSysConfig = document.querySelector('[data-admin-action="add-sysconfig"]');
    if (addSysConfig) {
      addSysConfig.addEventListener('click', addSysConfigItem);
    }

    var refreshScene = document.querySelector('[data-admin-action="refresh-scene"]');
    if (refreshScene) {
      refreshScene.addEventListener('click', loadSceneModule);
    }

    // World notices
    var publishNoticeBtn = document.querySelector('[data-admin-action="publish-world-notice"]');
    if (publishNoticeBtn) {
      publishNoticeBtn.addEventListener('click', publishWorldNotice);
    }

    // Safety advisories
    var publishAdvisoryBtn = document.querySelector('[data-admin-action="publish-safety-advisory"]');
    if (publishAdvisoryBtn) {
      publishAdvisoryBtn.addEventListener('click', publishSafetyAdvisory);
    }

    var refreshAdvisoriesBtn = document.querySelector('[data-admin-action="refresh-safety-advisories"]');
    if (refreshAdvisoriesBtn) {
      refreshAdvisoriesBtn.addEventListener('click', function () { loadSafetyData(); });
    }

    var refreshReportsBtn = document.querySelector('[data-admin-action="refresh-safety-reports"]');
    if (refreshReportsBtn) {
      refreshReportsBtn.addEventListener('click', function () { loadSafetyData(); });
    }

    var refreshSanctionsBtn = document.querySelector('[data-admin-action="refresh-resident-sanctions"]');
    if (refreshSanctionsBtn) {
      refreshSanctionsBtn.addEventListener('click', function () { loadSafetyData(); });
    }

  }

  // ====== System Config (Gateway read/write) ======

  var sysConfigCache = {};

  async function loadSysConfig() {
    var statusEl = document.getElementById('sysConfigGatewayStatus');
    var editorEl = document.getElementById('sysConfigEditor');
    if (!editorEl) return;

    if (!gatewayUrl) {
      if (statusEl) { statusEl.textContent = 'Gateway 未连接'; statusEl.style.color = 'var(--ds-text-danger)'; }
      clear(editorEl);
      editorEl.appendChild(el('p', { style: 'color:var(--ds-text-danger);' }, '请先通过 ?gateway= 参数连接 Gateway'));
      return;
    }

    if (statusEl) { statusEl.textContent = '加载中...'; statusEl.style.color = 'var(--ds-text-secondary)'; }

    var config = await fetchGatewayJson('/v1/admin/config');
    if (config && typeof config === 'object' && !Array.isArray(config)) {
      sysConfigCache = config;
      if (statusEl) { statusEl.textContent = '已同步 ' + Object.keys(config).length + ' 个参数'; statusEl.style.color = 'var(--ds-color-success)'; }
      renderSysConfigEditor(config);
    } else {
      if (statusEl) { statusEl.textContent = '加载失败'; statusEl.style.color = 'var(--ds-text-danger)'; }
      clear(editorEl);
      editorEl.appendChild(el('p', { style: 'color:var(--ds-text-danger);' }, '无法从 Gateway 读取配置，请确认 Gateway 已启动且 /v1/admin/config 端点可用。'));
    }
  }

  function renderSysConfigEditor(config) {
    var editorEl = document.getElementById('sysConfigEditor');
    if (!editorEl) return;
    clear(editorEl);

    var keys = Object.keys(config);
    if (!keys.length) {
      editorEl.appendChild(el('p', { style: 'color:var(--ds-text-muted);' }, '暂无系统参数，请使用下方表单添加。'));
      return;
    }

    keys.sort();
    for (var i = 0; i < keys.length; i++) {
      (function (key, value) {
        var row = el('div', { class: 'ds-config-item', style: 'display:flex;align-items:center;gap:0.75rem;padding:0.5rem 0;border-bottom:1px solid var(--ds-border-light);' });

        var keyInput = el('input', { type: 'text', value: key, style: 'width:180px;font-family:var(--ds-font-mono);font-size:12px;' });
        keyInput.readOnly = true;
        row.appendChild(keyInput);

        var valueInput = el('input', { type: 'text', value: value, style: 'flex:1;font-family:var(--ds-font-mono);font-size:12px;' });
        row.appendChild(valueInput);

        var saveBtn = el('button', { class: 'ds-btn-primary ds-btn-xs' }, '保存');
        saveBtn.addEventListener('click', function () {
          saveSysConfigItem(keyInput.value, valueInput.value, saveBtn);
        });
        row.appendChild(saveBtn);

        editorEl.appendChild(row);
      })(keys[i], config[keys[i]]);
    }
  }

  async function saveSysConfigItem(key, value, btnEl) {
    if (!key.trim()) { showAdminNotice('参数键名不能为空', 'error'); return; }
    if (btnEl) { btnEl.disabled = true; btnEl.textContent = '保存中...'; }

    var result = await fetchGatewayJsonPost('/v1/admin/config', { config: (function () { var o = {}; o[key] = value; return o; })() });

    if (btnEl) { btnEl.disabled = false; btnEl.textContent = '保存'; }

    if (result.error) {
      showAdminNotice('保存失败: ' + result.error, 'error');
    } else if (result.ok) {
      sysConfigCache[key] = value;
      showAdminNotice('参数 ' + key + ' 已保存', 'success');
    } else {
      showAdminNotice('保存失败 (HTTP ' + result.status + '): ' + JSON.stringify(result.data), 'error');
    }
  }

  async function addSysConfigItem() {
    var keyInput = document.getElementById('sysConfigNewKey');
    var valueInput = document.getElementById('sysConfigNewValue');
    if (!keyInput || !valueInput) return;

    var key = keyInput.value.trim();
    var value = valueInput.value.trim();

    if (!key) { showAdminNotice('请输入参数键名', 'error'); return; }

    var addBtn = document.querySelector('[data-admin-action="add-sysconfig"]');
    if (addBtn) { addBtn.disabled = true; addBtn.textContent = '添加中...'; }

    var result = await fetchGatewayJsonPost('/v1/admin/config', { config: (function () { var o = {}; o[key] = value; return o; })() });

    if (addBtn) { addBtn.disabled = false; addBtn.textContent = '添加参数'; }

    if (result.error) {
      showAdminNotice('添加失败: ' + result.error, 'error');
    } else if (result.ok) {
      sysConfigCache[key] = value;
      keyInput.value = '';
      valueInput.value = '';
      showAdminNotice('参数 ' + key + ' 已添加', 'success');
      renderSysConfigEditor(sysConfigCache);
    } else {
      showAdminNotice('添加失败 (HTTP ' + result.status + '): ' + JSON.stringify(result.data), 'error');
    }
  }

  // ====== World Notices ======

  async function loadWorldNotices() {
    if (!gatewayUrl) {
      worldNotices = [];
      renderWorldNotices();
      return;
    }
    try {
      var result = await fetchGatewayJson('/v1/world-square');
      if (Array.isArray(result)) {
        worldNotices = result;
      } else {
        worldNotices = [];
        showAdminNotice('Gateway 世界公告读取失败，已显示空态', 'error');
      }
      renderWorldNotices();
    } catch (e) {
      console.warn('admin-ds load world notices failed', e);
      worldNotices = [];
      renderWorldNotices();
      showAdminNotice('Gateway 世界公告读取失败，已显示空态', 'error');
    }
  }

  function renderWorldNotices() {
    var tbody = document.getElementById('worldNoticeTableBody');
    if (!tbody) return;
    clear(tbody);

    if (!worldNotices.length) {
      renderEmptyRow(tbody, 6, '暂无世界公告');
      return;
    }

    var severityMap = { info: '信息', warning: '警告', critical: '严重' };
    var severityTag = { info: 'info', warning: 'warning', critical: 'error' };

    for (var i = 0; i < worldNotices.length; i++) {
      (function (notice) {
        var tr = el('tr');
        var timeStr = new Date(notice.posted_at_ms).toLocaleString('zh-CN');
        tr.appendChild(makeTd(timeStr, 'font-family:var(--ds-font-mono);font-size:12px;color:var(--ds-text-secondary);'));
        tr.appendChild(makeTd(notice.title));
        var tdBody = el('td');
        tdBody.style.cssText = 'max-width:240px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        tdBody.textContent = notice.body;
        tr.appendChild(tdBody);
        var tdSev = el('td');
        var sevLabel = severityMap[notice.severity] || notice.severity;
        var sevTag = severityTag[notice.severity] || 'default';
        tdSev.appendChild(makeTag(sevLabel, sevTag));
        tr.appendChild(tdSev);
        tr.appendChild(makeTd(notice.author_id));
        var tdTags = el('td');
        tdTags.textContent = Array.isArray(notice.tags) ? notice.tags.join(', ') : '';
        tdTags.style.cssText = 'color:var(--ds-text-secondary);font-size:12px;';
        tr.appendChild(tdTags);
        tbody.appendChild(tr);
      })(worldNotices[i]);
    }
  }

  async function publishWorldNotice() {
    var titleEl = document.getElementById('worldNoticeTitle');
    var bodyEl = document.getElementById('worldNoticeBody');
    var severityEl = document.getElementById('worldNoticeSeverity');
    var tagsEl = document.getElementById('worldNoticeTags');
    var btn = document.querySelector('[data-admin-action="publish-world-notice"]');

    var title = (titleEl && titleEl.value || '').trim();
    var body = (bodyEl && bodyEl.value || '').trim();
    if (!title || !body) { showAdminNotice('标题和正文不能为空', 'error'); return; }

    var tagsRaw = (tagsEl && tagsEl.value || '').trim();
    var tags = tagsRaw ? tagsRaw.split(',').map(function (s) { return s.trim(); }).filter(Boolean) : [];

    if (btn) { btn.disabled = true; btn.textContent = '发布中...'; }

    var result = await fetchGatewayJsonPost('/v1/world-square/notices', {
      actor_id: currentGatewayIdentity(),
      title: title,
      body: body,
      severity: severityEl ? severityEl.value : 'info',
      tags: tags.length ? tags : null
    });

    if (btn) { btn.disabled = false; btn.textContent = '发布'; }

    if (result.error) {
      showAdminNotice('发布失败: ' + result.error, 'error');
    } else if (result.ok) {
      showAdminNotice('公告已发布', 'success');
      if (titleEl) titleEl.value = '';
      if (bodyEl) bodyEl.value = '';
      if (tagsEl) tagsEl.value = '';
      loadWorldNotices();
    } else {
      showAdminNotice('发布失败 (HTTP ' + result.status + ')', 'error');
    }
  }

  // ====== Safety Advisories ======

  async function loadSafetyData() {
    if (!gatewayUrl) {
      safetyAdvisories = [];
      safetyReports = [];
      residentSanctions = [];
      renderSafetyAdvisories();
      renderSafetyReports();
      renderResidentSanctions();
      return;
    }
    try {
      var result = await fetchGatewayJson('/v1/world-safety');
      var safetyReadOk = result && typeof result === 'object' &&
        Array.isArray(result.advisories) &&
        Array.isArray(result.reports) &&
        Array.isArray(result.resident_sanctions);
      if (safetyReadOk) {
        safetyAdvisories = Array.isArray(result.advisories) ? result.advisories : [];
        safetyReports = Array.isArray(result.reports) ? result.reports : [];
        residentSanctions = Array.isArray(result.resident_sanctions) ? result.resident_sanctions : [];
      } else {
        safetyAdvisories = [];
        safetyReports = [];
        residentSanctions = [];
        showAdminNotice('Gateway 安全治理读取失败，已显示空态', 'error');
      }
      renderSafetyAdvisories();
      renderSafetyReports();
      renderResidentSanctions();
    } catch (e) {
      console.warn('admin-ds load safety data failed', e);
      safetyAdvisories = [];
      safetyReports = [];
      residentSanctions = [];
      renderSafetyAdvisories();
      renderSafetyReports();
      renderResidentSanctions();
      showAdminNotice('Gateway 安全治理读取失败，已显示空态', 'error');
    }
  }

  function renderSafetyAdvisories() {
    var tbody = document.getElementById('safetyAdvisoryTableBody');
    if (!tbody) return;
    clear(tbody);

    if (!safetyAdvisories.length) {
      renderEmptyRow(tbody, 6, '暂无安全通告');
      return;
    }

    var actionTag = { warn: 'warning', restrict: 'error', block: 'error', monitor: 'info' };
    var actionText = { warn: '警告', restrict: '限制', block: '封禁', monitor: '监控' };

    for (var i = 0; i < safetyAdvisories.length; i++) {
      (function (adv) {
        var tr = el('tr');
        var timeStr = new Date(adv.issued_at_ms).toLocaleString('zh-CN');
        tr.appendChild(makeTd(timeStr, 'font-family:var(--ds-font-mono);font-size:12px;color:var(--ds-text-secondary);'));
        tr.appendChild(makeTd(adv.subject_kind));
        tr.appendChild(makeTd(adv.subject_ref, 'font-family:var(--ds-font-mono);font-size:12px;'));
        var tdAction = el('td');
        tdAction.appendChild(makeTag(actionText[adv.action] || adv.action, actionTag[adv.action] || 'default'));
        tr.appendChild(tdAction);
        tr.appendChild(makeTd(adv.reason));
        tr.appendChild(makeTd(adv.issued_by));
        tbody.appendChild(tr);
      })(safetyAdvisories[i]);
    }
  }

  function renderSafetyReports() {
    var tbody = document.getElementById('safetyReportTableBody');
    if (!tbody) return;
    clear(tbody);

    if (!safetyReports.length) {
      renderEmptyRow(tbody, 6, '暂无安全举报');
      return;
    }

    var statusTag = { Submitted: 'warning', Reviewing: 'info', Resolved: 'success', Dismissed: 'default' };
    var statusText = { Submitted: '待审', Reviewing: '审核中', Resolved: '已解决', Dismissed: '已驳回' };

    for (var i = 0; i < safetyReports.length; i++) {
      (function (report) {
        var tr = el('tr');
        tr.appendChild(makeTd(report.report_id, 'font-family:var(--ds-font-mono);font-size:11px;'));
        tr.appendChild(makeTd((report.target_kind || '') + ': ' + (report.target_ref || '')));
        tr.appendChild(makeTd(report.reporter_id));
        tr.appendChild(makeTd(report.summary, 'max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;'));
        var tdStatus = el('td');
        var stLabel = statusText[report.status] || report.status;
        var stTag = statusTag[report.status] || 'default';
        tdStatus.appendChild(makeTag(stLabel, stTag));
        tr.appendChild(tdStatus);

        var tdActions = el('td');
        var btnGroup = makeBtnGroup();
        if (report.status === 'Submitted' || report.status === 'Reviewing') {
          var resolveBtn = makeBtn('解决', 'ds-btn-primary ds-btn-xs');
          resolveBtn.addEventListener('click', function (e) {
            e.stopPropagation();
            reviewSafetyReport(report.report_id, 'Resolved', resolveBtn);
          });
          btnGroup.appendChild(resolveBtn);
          var dismissBtn = makeBtn('驳回', 'ds-btn-danger-text ds-btn-xs');
          dismissBtn.addEventListener('click', function (e) {
            e.stopPropagation();
            reviewSafetyReport(report.report_id, 'Dismissed', dismissBtn);
          });
          btnGroup.appendChild(dismissBtn);
        }
        tdActions.appendChild(btnGroup);
        tr.appendChild(tdActions);

        tbody.appendChild(tr);
      })(safetyReports[i]);
    }
  }

  async function reviewSafetyReport(reportId, status, btn) {
    if (!reportId) return;
    if (btn) { btn.disabled = true; btn.textContent = '...'; }
    var result = await fetchGatewayJsonPost('/v1/world-safety/reports/review', {
      actor_id: currentGatewayIdentity(),
      report_id: reportId,
      status: status,
      resolution: status === 'Resolved' ? '已处理' : '证据不足'
    });
    if (btn) { btn.disabled = false; btn.textContent = status === 'Resolved' ? '解决' : '驳回'; }
    if (result.error) {
      showAdminNotice('操作失败: ' + result.error, 'error');
    } else if (result.ok) {
      showAdminNotice('举报 ' + reportId + ' 已' + (status === 'Resolved' ? '解决' : '驳回'), 'success');
      loadSafetyData();
    } else {
      showAdminNotice('操作失败 (HTTP ' + result.status + ')', 'error');
    }
  }

  function renderResidentSanctions() {
    var tbody = document.getElementById('sanctionTableBody');
    if (!tbody) return;
    clear(tbody);

    if (!residentSanctions.length) {
      renderEmptyRow(tbody, 5, '暂无居民制裁');
      return;
    }

    var statusTag = { Active: 'error', Lifted: 'success' };
    var statusText = { Active: '生效中', Lifted: '已解除' };

    for (var i = 0; i < residentSanctions.length; i++) {
      (function (sanction) {
        var tr = el('tr');
        tr.appendChild(makeTd(sanction.resident_id, 'font-family:var(--ds-font-mono);font-size:12px;'));
        tr.appendChild(makeTd(sanction.reason));
        var tdStatus = el('td');
        var sl = statusText[sanction.status] || sanction.status;
        var sc = statusTag[sanction.status] || 'default';
        tdStatus.appendChild(makeTag(sl, sc));
        tr.appendChild(tdStatus);
        tr.appendChild(makeTd(sanction.issued_by));
        var tdActions = el('td');
        var btnGroup = makeBtnGroup();
        if (sanction.status === 'Active') {
          var liftBtn = makeBtn('解除制裁', 'ds-btn-outline ds-btn-xs');
          liftBtn.addEventListener('click', function (e) {
            e.stopPropagation();
            unsanctionResident(sanction.sanction_id, liftBtn);
          });
          btnGroup.appendChild(liftBtn);
        }
        tdActions.appendChild(btnGroup);
        tr.appendChild(tdActions);
        tbody.appendChild(tr);
      })(residentSanctions[i]);
    }
  }

  async function unsanctionResident(sanctionId, btn) {
    if (!sanctionId) { showAdminNotice('缺少制裁 ID', 'error'); return; }
    if (!gatewayUrl) { showAdminNotice('Gateway 未连接，无法执行解除制裁', 'error'); return; }
    setBtnLoading(btn, true);
    try {
      var result = await fetchGatewayJsonPost('/v1/admin/residents/unsanction', {
        actor_id: currentGatewayIdentity(),
        sanction_id: sanctionId
      });
      if (result.error) {
        setBtnResult(btn, false, result.error);
        showAdminNotice('解除制裁失败: ' + result.error, 'error');
      } else if (result.ok) {
        setBtnResult(btn, true);
        showAdminNotice('制裁 ' + sanctionId + ' 已解除', 'success');
        loadSafetyData();
      } else {
        setBtnResult(btn, false, 'HTTP ' + result.status);
        showAdminNotice('解除制裁失败 (HTTP ' + result.status + ')', 'error');
      }
    } catch (e) {
      setBtnResult(btn, false, e.message);
      showAdminNotice('网络错误: ' + e.message, 'error');
    }
  }

  async function publishSafetyAdvisory() {
    var kindEl = document.getElementById('safetyAdvisorySubjectKind');
    var refEl = document.getElementById('safetyAdvisorySubjectRef');
    var actionEl = document.getElementById('safetyAdvisoryAction');
    var reasonEl = document.getElementById('safetyAdvisoryReason');
    var btn = document.querySelector('[data-admin-action="publish-safety-advisory"]');

    var subjectKind = kindEl ? kindEl.value : 'resident';
    var subjectRef = (refEl && refEl.value || '').trim();
    var action = actionEl ? actionEl.value : 'warn';
    var reason = (reasonEl && reasonEl.value || '').trim();

    if (!subjectRef || !reason) { showAdminNotice('目标 ID 和原因不能为空', 'error'); return; }

    if (btn) { btn.disabled = true; btn.textContent = '发布中...'; }

    var result = await fetchGatewayJsonPost('/v1/world-safety/advisories', {
      actor_id: currentGatewayIdentity(),
      subject_kind: subjectKind,
      subject_ref: subjectRef,
      action: action,
      reason: reason
    });

    if (btn) { btn.disabled = false; btn.textContent = '发布'; }

    if (result.error) {
      showAdminNotice('发布失败: ' + result.error, 'error');
    } else if (result.ok) {
      showAdminNotice('安全通告已发布', 'success');
      if (refEl) refEl.value = '';
      if (reasonEl) reasonEl.value = '';
      loadSafetyData();
    } else {
      showAdminNotice('发布失败 (HTTP ' + result.status + ')', 'error');
    }
  }

  // ====== Admin Resident Ban / Unban (real Gateway POST) ======

  function setBtnLoading(btn, loading) {
    if (!btn) return;
    if (loading) {
      btn.disabled = true;
      btn._prevText = btn.textContent;
      btn.textContent = '处理中...';
      btn.style.opacity = '0.6';
    } else {
      btn.disabled = false;
      if (btn._prevText) { btn.textContent = btn._prevText; delete btn._prevText; }
      btn.style.opacity = '';
    }
  }

  function setBtnResult(btn, ok, message) {
    if (!btn) return;
    btn.disabled = false;
    btn.style.opacity = '';
    if (ok) {
      btn.textContent = '已完成';
      btn.classList.add('ds-btn-success-tick');
      setTimeout(function () {
        btn.classList.remove('ds-btn-success-tick');
        if (btn._prevText) { btn.textContent = btn._prevText; delete btn._prevText; }
      }, 2000);
    } else {
      btn.textContent = '操作失败';
      btn.title = message || '';
      btn.classList.add('ds-btn-error-flash');
      setTimeout(function () {
        btn.classList.remove('ds-btn-error-flash');
        if (btn._prevText) { btn.textContent = btn._prevText; delete btn._prevText; }
        btn.title = '';
      }, 2500);
    }
  }

  async function banResident(residentId, btn) {
    if (!residentId) { showAdminNotice('缺少居民 ID', 'error'); return; }
    if (!gatewayUrl) { showAdminNotice('Gateway 未连接，无法执行禁用操作', 'error'); return; }
    var reason = prompt('请输入禁用理由：', '违规行为');
    if (reason === null) { setBtnResult(btn, false, '已取消'); return; }
    setBtnLoading(btn, true);
    try {
      var result = await fetchGatewayJsonPost('/v1/admin/residents/ban', {
        resident_id: residentId,
        reason: reason || '违规行为',
        actor_id: currentGatewayIdentity()
      });
      if (result.error) {
        setBtnResult(btn, false, result.error);
        showAdminNotice('禁用失败: ' + result.error, 'error');
      } else if (result.ok) {
        setBtnResult(btn, true);
        showAdminNotice('居民 ' + residentId + ' 已禁用', 'success');
        renderResidents(currentResidentFilter(), currentResidentRoleFilter(), currentResidentSearchTerm());
      } else {
        setBtnResult(btn, false, 'HTTP ' + result.status);
        showAdminNotice('禁用失败 (HTTP ' + result.status + ')', 'error');
      }
    } catch (e) {
      setBtnResult(btn, false, e.message);
      showAdminNotice('网络错误: ' + e.message, 'error');
    }
  }

  async function unbanResident(residentId, btn) {
    if (!residentId) { showAdminNotice('缺少居民 ID', 'error'); return; }
    if (!gatewayUrl) { showAdminNotice('Gateway 未连接，无法执行恢复操作', 'error'); return; }
    setBtnLoading(btn, true);
    try {
      var result = await fetchGatewayJsonPost('/v1/admin/residents/unban', {
        resident_id: residentId,
        actor_id: currentGatewayIdentity()
      });
      if (result.error) {
        setBtnResult(btn, false, result.error);
        showAdminNotice('恢复失败: ' + result.error, 'error');
      } else if (result.ok) {
        setBtnResult(btn, true);
        showAdminNotice('居民 ' + residentId + ' 已恢复', 'success');
        renderResidents(currentResidentFilter(), currentResidentRoleFilter(), currentResidentSearchTerm());
      } else {
        setBtnResult(btn, false, 'HTTP ' + result.status);
        showAdminNotice('恢复失败 (HTTP ' + result.status + ')', 'error');
      }
    } catch (e) {
      setBtnResult(btn, false, e.message);
      showAdminNotice('网络错误: ' + e.message, 'error');
    }
  }

  // ====== Admin Room Freeze / Unfreeze (real Gateway POST) ======

  async function freezeRoom(roomId, btn) {
    if (!roomId) { showAdminNotice('缺少房间 ID', 'error'); return; }
    if (!gatewayUrl) { showAdminNotice('Gateway 未连接，无法执行冻结操作', 'error'); return; }
    setBtnLoading(btn, true);
    try {
      var result = await fetchGatewayJsonPost('/v1/admin/rooms/freeze', { room_id: roomId });
      if (result.error) {
        setBtnResult(btn, false, result.error);
        showAdminNotice('冻结失败: ' + result.error, 'error');
      } else if (result.ok) {
        setBtnResult(btn, true);
        showAdminNotice('房间 ' + roomId + ' 已冻结', 'success');
        renderRooms(currentRoomTypeFilter(), currentRoomSearchTerm());
      } else {
        setBtnResult(btn, false, 'HTTP ' + result.status);
        showAdminNotice('冻结失败 (HTTP ' + result.status + ')', 'error');
      }
    } catch (e) {
      setBtnResult(btn, false, e.message);
      showAdminNotice('网络错误: ' + e.message, 'error');
    }
  }

  async function unfreezeRoom(roomId, btn) {
    if (!roomId) { showAdminNotice('缺少房间 ID', 'error'); return; }
    if (!gatewayUrl) { showAdminNotice('Gateway 未连接，无法执行解冻操作', 'error'); return; }
    setBtnLoading(btn, true);
    try {
      var result = await fetchGatewayJsonPost('/v1/admin/rooms/unfreeze', { room_id: roomId });
      if (result.error) {
        setBtnResult(btn, false, result.error);
        showAdminNotice('解冻失败: ' + result.error, 'error');
      } else if (result.ok) {
        setBtnResult(btn, true);
        showAdminNotice('房间 ' + roomId + ' 已解冻', 'success');
        renderRooms(currentRoomTypeFilter(), currentRoomSearchTerm());
      } else {
        setBtnResult(btn, false, 'HTTP ' + result.status);
        showAdminNotice('解冻失败 (HTTP ' + result.status + ')', 'error');
      }
    } catch (e) {
      setBtnResult(btn, false, e.message);
      showAdminNotice('网络错误: ' + e.message, 'error');
    }
  }

  function currentRoomTypeFilter() { return document.getElementById('roomTypeFilter')?.value || 'all'; }
  function currentRoomSearchTerm() { return document.getElementById('roomSearch')?.value || ''; }

  function currentResidentSearchTerm() { return document.getElementById('residentSearch')?.value || ''; }

  // ====== Dashboard live time ======
  function updateDashboardTime() {
    if (dashboardTime) {
      var now = new Date();
      dashboardTime.textContent = now.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }
  }
  updateDashboardTime();
  setInterval(updateDashboardTime, 30000);

  // ====== Initial Render ======
  bindStaticAdminActions();
  renderResidents('all', 'all', '');
  renderRooms('all', '');
  renderMessages('all', 'all', '');
  renderInvites();
  renderLogs('all', 'all', '');
  updateDashboardSummary('local');
  // The standalone auth surface lives in a module script loaded after this
  // file. Expose only the refresh boundary so a successful login/logout can
  // rehydrate the same Gateway-owned projection without a page reload.
  window.__adminDsRefresh = loadGatewayAdminData;
  loadGatewayAdminData();

  // ====== Keyboard shortcuts ======
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { closeDetail(); }
    if (e.ctrlKey && e.key === 'b') {
      e.preventDefault();
      sidebarToggle.click();
    }
  });

  if (debugEnabled) {
    console.log('AJW聊天 · 正式管理后台已就绪');
    console.log('模块: 仪表盘 | 居民管理 | 会话与房间 | 消息审核 | 权限与邀请 | 系统配置 | 日志与告警');
    console.log('快捷键: Esc 关闭详情 | Ctrl+B 切换侧栏');
  }
})();
