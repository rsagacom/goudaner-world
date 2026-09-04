import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";

const WEB_SHELL_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export const APP_LOCAL_IMPORT_PATHS = Object.freeze([
  "./composer-state.js",
  "./shell-caretaker-panel.js",
  "./shell-composer.js",
  "./shell-composer-symbols.js",
  "./shell-avatar.js",
  "./shell-dom-helpers.js",
  "./shell-errors.js",
  "./shell-export-utils.js",
  "./shell-gateway.js",
  "./shell-gateway-polling.js",
  "./shell-gateway-realtime.js",
  "./shell-gateway-sync.js",
  "./shell-lifecycle.js",
  "./shell-governance-status.js",
  "./shell-governance-render.js",
  "./shell-private-room-locked-card.js",
  "./shell-governance-city-surfaces.js",
  "./shell-resident-surfaces.js",
  "./shell-room-list-surfaces.js",
  "./shell-room-digest-surfaces.js",
  "./shell-thread-status-surfaces.js",
  "./shell-world-surfaces.js",
  "./shell-identity.js",
  "./shell-labels.js",
  "./shell-personal-room-policy.js",
  "./shell-message-state.js",
  "./shell-message-send.js",
  "./shell-message-render.js",
  "./shell-message-search.js",
  "./shell-quick-action-labels.js",
  "./shell-quick-action-templates.js",
  "./shell-quick-action-preview.js",
  "./shell-quick-action-reader.js",
  "./shell-room-render.js",
  "./shell-room-stage.js",
  "./shell-quick-actions.js",
  "./shell-payload.js",
  "./shell-state-normalize.js",
  "./shell-state.js",
  "./shell-chat-focus.js",
  "./shell-timeline-empty-state.js",
  "./shell-scene-runtime.js",
  "./shell-scene-image-layer.js",
  "./shell-message-body.js",
  "./shell-image-compress.js",
  "./shell-attachment-lightbox.js",
  "./shell-room-summary.js",
  "./shell-room-context.js",
  "./shell-auth.js",
  "./shell-room-profiles.js",
  "./shell-shared.js",
  "./shell-storage-keys.js",
  "./pretext-stage.js",
  "./shell-room-rail.js",
  "./shell-user-detail-card.js",
  "./shell-conversation-callout.js",
  "./shell-message-action-payload.js",
  "./shell-message-action-sheet.js",
  "./shell-mode-view.js",
  "./shell-chrome-text.js",
  "./shell-scene-chrome.js",
  "./shell-caretaker-dom.js",
  "./shell-conversation-callout-render.js",
]);

function appLocalImportUrlMap() {
  return new Map(
    APP_LOCAL_IMPORT_PATHS.map((relativePath) => [
      relativePath,
      new URL(`..${relativePath.slice(1)}`, import.meta.url).href,
    ]),
  );
}

export function rewriteAppLocalImports(source, moduleUrls = appLocalImportUrlMap()) {
  if (typeof source !== "string" || !source) return "";
  return source.replace(/from\s+(["'])(\.\/[^"']+\.js)\1/g, (match, quote, relativePath) => {
    const moduleUrl = moduleUrls.get(relativePath);
    return moduleUrl ? `from ${quote}${moduleUrl}${quote}` : match;
  });
}

class FakeEvent {
  constructor(type, init = {}) {
    this.type = type;
    this.bubbles = Boolean(init.bubbles);
    this.cancelable = Boolean(init.cancelable);
    this.defaultPrevented = false;
    this.target = null;
    this.currentTarget = null;
    this.key = init.key;
    this.shiftKey = Boolean(init.shiftKey);
    this.altKey = Boolean(init.altKey);
    this.metaKey = Boolean(init.metaKey);
    this.ctrlKey = Boolean(init.ctrlKey);
    this.isComposing = Boolean(init.isComposing);
    this.repeat = Boolean(init.repeat);
  }

  preventDefault() {
    if (this.cancelable) {
      this.defaultPrevented = true;
    }
  }

  stopPropagation() {}
}

class FakeClassList {
  constructor(owner) {
    this.owner = owner;
    this.tokens = new Set();
  }

  add(...tokens) {
    for (const token of tokens) this.tokens.add(token);
    this.sync();
  }

  remove(...tokens) {
    for (const token of tokens) this.tokens.delete(token);
    this.sync();
  }

  toggle(token, force) {
    if (force === true) {
      this.tokens.add(token);
      this.sync();
      return true;
    }
    if (force === false) {
      this.tokens.delete(token);
      this.sync();
      return false;
    }
    if (this.tokens.has(token)) {
      this.tokens.delete(token);
      this.sync();
      return false;
    }
    this.tokens.add(token);
    this.sync();
    return true;
  }

  contains(token) {
    return this.tokens.has(token);
  }

  toString() {
    return Array.from(this.tokens).join(" ");
  }

  fromString(value) {
    this.tokens = new Set(String(value || "").split(/\s+/).filter(Boolean));
    this.sync();
  }

  sync() {
    this.owner._className = this.toString();
    this.owner._attributes.set("class", this.owner._className);
  }
}

class FakeElement {
  constructor(tagName, ownerDocument) {
    this.tagName = String(tagName).toUpperCase();
    this.ownerDocument = ownerDocument;
    this.parentNode = null;
    this.children = [];
    this._textContent = "";
    this._innerHTML = "";
    this._className = "";
    this._attributes = new Map();
    this.dataset = new Proxy(
      {},
      {
        get: (_, key) => this._attributes.get(`data-${toKebab(String(key))}`),
        set: (_, key, value) => {
          this.setAttribute(`data-${toKebab(String(key))}`, String(value));
          return true;
        },
        deleteProperty: (_, key) => {
          this.removeAttribute(`data-${toKebab(String(key))}`);
          return true;
        },
      },
    );
    this.style = {};
    this.classList = new FakeClassList(this);
    this.eventListeners = new Map();
    this.disabled = false;
    this.value = "";
    this.placeholder = "";
    this.type = "";
    this.autocomplete = "";
    this.enterKeyHint = "";
    this.rows = 0;
    this.href = "";
    this.target = "";
    this.rel = "";
    this.ariaLabel = "";
    this.scrollHeight = 74;
    this._isFocused = false;
  }

  get id() {
    return this.getAttribute("id") || "";
  }

  set id(value) {
    this.setAttribute("id", value);
  }

  get className() {
    return this._className;
  }

  set className(value) {
    this.classList.fromString(value);
  }

  get textContent() {
    const childText = this.children.map((child) => child.textContent).join("");
    return `${this._textContent}${childText}`;
  }

  set textContent(value) {
    this._textContent = String(value ?? "");
    this._innerHTML = this._textContent;
    this.children = [];
  }

  get innerHTML() {
    return this._innerHTML;
  }

  set innerHTML(value) {
    this._innerHTML = String(value ?? "");
    this._textContent = stripTags(this._innerHTML);
    this.children = [];
  }

  get isConnected() {
    let node = this;
    while (node) {
      if (node === this.ownerDocument.body) return true;
      node = node.parentNode;
    }
    return false;
  }

  get firstChild() {
    return this.children[0] || null;
  }

  get lastChild() {
    return this.children[this.children.length - 1] || null;
  }

  appendChild(child) {
    if (typeof child === "string") {
      child = this.ownerDocument.createTextNode(child);
    }
    if (child.parentNode) {
      child.parentNode.removeChild(child);
    }
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  append(...nodes) {
    for (const node of nodes) this.appendChild(node);
  }

  prepend(...nodes) {
    const items = nodes.map((node) => (typeof node === "string" ? this.ownerDocument.createTextNode(node) : node));
    for (const node of items.reverse()) {
      if (node.parentNode) {
        node.parentNode.removeChild(node);
      }
      node.parentNode = this;
      this.children.unshift(node);
    }
  }

  replaceChildren(...nodes) {
    this.children = [];
    this._textContent = "";
    this._innerHTML = "";
    this.append(...nodes);
  }

  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index >= 0) {
      this.children.splice(index, 1);
      child.parentNode = null;
    }
    return child;
  }

  remove() {
    if (this.parentNode) {
      this.parentNode.removeChild(this);
    }
  }

  insertBefore(newNode, referenceNode) {
    if (typeof newNode === "string") {
      newNode = this.ownerDocument.createTextNode(newNode);
    }
    if (newNode.parentNode) {
      newNode.parentNode.removeChild(newNode);
    }
    const index = referenceNode ? this.children.indexOf(referenceNode) : -1;
    if (index < 0) {
      this.children.push(newNode);
    } else {
      this.children.splice(index, 0, newNode);
    }
    newNode.parentNode = this;
    return newNode;
  }

  insertAdjacentElement(position, element) {
    if (position === "beforebegin") {
      if (!this.parentNode) return null;
      return this.parentNode.insertBefore(element, this);
    }
    if (position === "afterend") {
      if (!this.parentNode) return null;
      const siblings = this.parentNode.children;
      const index = siblings.indexOf(this);
      if (element.parentNode) {
        element.parentNode.removeChild(element);
      }
      siblings.splice(index + 1, 0, element);
      element.parentNode = this.parentNode;
      return element;
    }
    if (position === "afterbegin") {
      return this.insertBefore(element, this.children[0] || null);
    }
    return this.appendChild(element);
  }

  setAttribute(name, value) {
    const stringValue = String(value);
    this._attributes.set(name, stringValue);
    if (name === "class") {
      this.classList.fromString(stringValue);
      return;
    }
    if (name === "id") {
      this.ownerDocument._indexById(stringValue, this);
      return;
    }
    if (name === "aria-label") {
      this.ariaLabel = stringValue;
      return;
    }
    if (name === "type") {
      this.type = stringValue;
      return;
    }
    if (name === "placeholder") {
      this.placeholder = stringValue;
      return;
    }
    if (name === "value") {
      this.value = stringValue;
      return;
    }
  }

  getAttribute(name) {
    return this._attributes.get(name) ?? null;
  }

  removeAttribute(name) {
    this._attributes.delete(name);
  }

  hasAttribute(name) {
    return this._attributes.has(name);
  }

  addEventListener(type, handler) {
    if (!this.eventListeners.has(type)) {
      this.eventListeners.set(type, []);
    }
    this.eventListeners.get(type).push(handler);
  }

  dispatchEvent(event) {
    event.target = event.target || this;
    event.currentTarget = this;
    const handlers = this.eventListeners.get(event.type) || [];
    for (const handler of handlers) {
      handler.call(this, event);
    }
    return !event.defaultPrevented;
  }

  click() {
    this.dispatchEvent(new FakeEvent("click", { bubbles: true, cancelable: true }));
  }

  focus() {
    this.ownerDocument.activeElement = this;
    this._isFocused = true;
  }

  select() {}

  reset() {
    const visit = (node) => {
      if ("value" in node) {
        node.value = "";
      }
      for (const child of node.children || []) {
        visit(child);
      }
    };
    visit(this);
  }

  scrollIntoView() {}

  getBoundingClientRect() {
    const width = Math.max(16, this.textContent.length * 8);
    const height = this.tagName === "CANVAS"
      ? Math.max(1, Number(this.height) || this.clientHeight || 128)
      : 24;
    return {
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: width,
      bottom: height,
      width,
      height,
    };
  }

  closest(selector) {
    let node = this;
    while (node) {
      if (matchesSelector(node, selector)) {
        return node;
      }
      node = node.parentNode;
    }
    return null;
  }

  querySelector(selector) {
    return this.ownerDocument._queryFrom(this, selector, true)[0] || null;
  }

  querySelectorAll(selector) {
    return this.ownerDocument._queryFrom(this, selector, false);
  }
}

class FakeTextNode extends FakeElement {
  constructor(text, ownerDocument) {
    super("#text", ownerDocument);
    this._textContent = String(text ?? "");
  }

  appendChild() {
    throw new Error("text nodes do not support children");
  }
}

class FakeCanvasElement extends FakeElement {
  constructor(ownerDocument) {
    super("canvas", ownerDocument);
    this.width = 0;
    this.height = 0;
    this.clientWidth = 360;
    this.clientHeight = 128;
    this._context = {
      font: '16px "Noto Sans SC", sans-serif',
      clearRect() {},
      fillRect() {},
      fillText() {},
      beginPath() {},
      closePath() {},
      moveTo() {},
      lineTo() {},
      arc() {},
      fill() {},
      stroke() {},
      setTransform() {},
      scale() {},
      measureText(text) {
        const font = typeof this.font === "string" ? this.font : "16px sans-serif";
        const sizeMatch = font.match(/(\d+(?:\.\d+)?)\s*px/);
        const size = sizeMatch ? Number(sizeMatch[1]) : 16;
        const value = String(text ?? "");
        return { width: Math.max(size * 0.6, value.length * size * 0.58) };
      },
    };
  }

  getContext(type) {
    if (type !== "2d") return null;
    return this._context;
  }
}

class FakeDocument {
  constructor() {
    this.title = "";
    this.activeElement = null;
    this._byId = new Map();
    this._listeners = new Map();
    this.body = new FakeElement("body", this);
    this.head = new FakeElement("head", this);
    this.documentElement = new FakeElement("html", this);
    this.documentElement.append(this.head, this.body);
  }

  createElement(tagName) {
    if (String(tagName).toLowerCase() === "canvas") {
      return new FakeCanvasElement(this);
    }
    return new FakeElement(tagName, this);
  }

  createTextNode(text) {
    return new FakeTextNode(text, this);
  }

  getElementById(id) {
    return this._byId.get(String(id)) || null;
  }

  querySelector(selector) {
    return this._queryFrom(this.body, selector, true)[0] || null;
  }

  querySelectorAll(selector) {
    return this._queryFrom(this.body, selector, false);
  }

  addEventListener(type, handler) {
    if (!this._listeners.has(type)) {
      this._listeners.set(type, []);
    }
    this._listeners.get(type).push(handler);
  }

  dispatchEvent(event) {
    const handlers = this._listeners.get(event.type) || [];
    for (const handler of handlers) handler.call(this, event);
  }

  _indexById(id, element) {
    this._byId.set(id, element);
  }

  _queryFrom(root, selector, firstOnly) {
    const selectors = selector.split(",").map((item) => item.trim()).filter(Boolean);
    const matches = [];
    const visit = (node) => {
      if (node !== root && selectors.some((item) => matchesSelector(node, item))) {
        matches.push(node);
        if (firstOnly) return true;
      }
      for (const child of node.children || []) {
        if (visit(child) && firstOnly) return true;
      }
      return false;
    };
    visit(root);
    return matches;
  }
}

function toKebab(value) {
  return value.replace(/[A-Z]/g, (match) => `-${match.toLowerCase()}`);
}

function stripTags(value) {
  return String(value || "").replace(/<[^>]*>/g, "");
}

function matchesSelector(node, selector) {
  if (!(node instanceof FakeElement)) return false;
  const value = selector.trim();
  if (!value) return false;
  const groups = value.split(",").map((item) => item.trim());
  return groups.some((group) => matchSingle(node, group));
}

function matchSingle(node, selector) {
  if (!selector) return false;
  let remaining = selector;
  let tag = null;
  const tagMatch = remaining.match(/^[a-zA-Z][a-zA-Z0-9-]*/);
  if (tagMatch) {
    tag = tagMatch[0].toLowerCase();
    remaining = remaining.slice(tagMatch[0].length);
    if (node.tagName.toLowerCase() !== tag) return false;
  }

  while (remaining.length) {
    if (remaining.startsWith(".")) {
      const classMatch = remaining.match(/^\.([a-zA-Z0-9_-]+)/);
      if (!classMatch) return false;
      if (!node.classList.contains(classMatch[1])) return false;
      remaining = remaining.slice(classMatch[0].length);
      continue;
    }
    if (remaining.startsWith("#")) {
      const idMatch = remaining.match(/^#([a-zA-Z0-9_-]+)/);
      if (!idMatch) return false;
      if (node.id !== idMatch[1]) return false;
      remaining = remaining.slice(idMatch[0].length);
      continue;
    }
    if (remaining.startsWith("[")) {
      const attrMatch = remaining.match(/^\[([^=\]]+)(?:=(["']?)(.*?)\2)?\]/);
      if (!attrMatch) return false;
      const attrName = attrMatch[1];
      const expected = attrMatch[3];
      const actual = node.getAttribute(attrName);
      if (expected === undefined || expected === "") {
        if (actual == null) return false;
      } else if (actual !== expected) {
        return false;
      }
      remaining = remaining.slice(attrMatch[0].length);
      continue;
    }
    return false;
  }

  return true;
}

function createPageNode(document, tagName, options = {}, children = []) {
  const el = document.createElement(tagName);
  if (options.id) el.id = options.id;
  if (options.className) el.className = options.className;
  if (options.textContent != null) el.textContent = options.textContent;
  if (options.dataset) {
    for (const [key, value] of Object.entries(options.dataset)) {
      el.dataset[key] = value;
    }
  }
  if (options.attrs) {
    for (const [key, value] of Object.entries(options.attrs)) {
      el.setAttribute(key, value);
    }
  }
  for (const child of children) {
    el.appendChild(child);
  }
  return el;
}

function makeInput(document, id, type, placeholder) {
  return createPageNode(
    document,
    "input",
    { id, attrs: { type, placeholder: placeholder || "" }, textContent: "", dataset: {} },
    [],
  );
}

function makeOption(document, value, text, { selected = false, disabled = false } = {}) {
  const option = createPageNode(document, "option", { attrs: { value }, textContent: text });
  option.value = value;
  option.selected = selected;
  option.disabled = disabled;
  if (selected) option.setAttribute("selected", "");
  if (disabled) option.setAttribute("disabled", "");
  return option;
}

function makeSelect(document, id, options, value) {
  const select = createPageNode(document, "select", { id });
  select.value = value || options.find((option) => option.selected)?.value || options[0]?.value || "";
  for (const option of options) {
    select.appendChild(makeOption(document, option.value, option.text, option));
  }
  return select;
}

async function readJsonFixture(relativePath) {
  const filePath = path.join(WEB_SHELL_ROOT, relativePath);
  const text = await fs.readFile(filePath, "utf8");
  return JSON.parse(text);
}

function responseFromJson(payload, { status = 200 } = {}) {
  return {
    ok: status >= 200 && status < 300,
    status,
    async json() {
      return structuredClone(payload);
    },
    async text() {
      return JSON.stringify(payload);
    },
  };
}

function responseNotFound() {
  return {
    ok: false,
    status: 404,
    async json() {
      return null;
    },
    async text() {
      return "";
    },
  };
}

function buildHubPage(document) {
  const body = document.body;
  body.dataset.shellPage = "hub";

  const app = createPageNode(document, "div", { className: "app" });
  const topbar = createPageNode(document, "header", { className: "topbar" }, [
    createPageNode(document, "div", { className: "topbar-main" }, [
      createPageNode(document, "div", { className: "masthead" }, [
        createPageNode(document, "div", {
          id: "masthead-eyebrow",
          className: "eyebrow",
          textContent: "我和狗蛋儿的家",
        }),
        createPageNode(document, "h1", {
          id: "masthead-title",
          textContent: "选一个房间开始",
        }),
        createPageNode(document, "div", { id: "entry-grid", className: "entry-grid" }, [
          createPageNode(document, "a", {
            className: "entry-card",
            dataset: { shellEntry: "user" },
            attrs: { href: "./user.html" },
          }, [
            createPageNode(document, "span", { className: "entry-badge", textContent: "用户房间" }),
            createPageNode(document, "strong", { textContent: "进入房间投影" }),
            createPageNode(document, "span", { textContent: "先看消息流，再看边缘功能。" }),
          ]),
          createPageNode(document, "a", {
            className: "entry-card",
            dataset: { shellEntry: "admin" },
            attrs: { href: "./admin.html" },
          }, [
            createPageNode(document, "span", { className: "entry-badge", textContent: "值守房间" }),
            createPageNode(document, "strong", { textContent: "进入值守投影" }),
            createPageNode(document, "span", { textContent: "先处理聊天，再看少量治理动作。" }),
          ]),
          createPageNode(document, "a", {
            className: "entry-card subtle",
            dataset: { shellEntry: "unified" },
            attrs: { href: "./unified.html" },
          }, [
            createPageNode(document, "span", { className: "entry-badge", textContent: "联调房间" }),
            createPageNode(document, "strong", { textContent: "打开联调投影" }),
            createPageNode(document, "span", { textContent: "同时看聊天和管理后台，便于本地排障。" }),
          ]),
        ]),
      ]),
    ]),
    createPageNode(document, "div", { className: "status-cluster" }, [
      createPageNode(document, "span", { id: "shell-mode-badge", className: "badge", textContent: "入口：房间门牌" }),
      createPageNode(document, "span", { id: "transport-state", className: "badge", textContent: "连接：启动中" }),
      createPageNode(document, "span", { id: "storage-state", className: "badge", textContent: "历史：检测中" }),
      createPageNode(document, "span", { id: "gateway-state", className: "badge", textContent: "网关：未连接" }),
      createPageNode(document, "span", { id: "provider-state", className: "badge", textContent: "来源：未知" }),
      createPageNode(document, "span", { id: "world-state", className: "badge", textContent: "扩展：离线" }),
    ]),
  ]);

  const layout = createPageNode(document, "main", { className: "layout" }, [
    createPageNode(document, "aside", { className: "sidebar-stack" }, [
      createPageNode(document, "section", { className: "panel guide-panel" }, [
        createPageNode(document, "div", { className: "panel-title", textContent: "如何开始" }),
        createPageNode(document, "div", {
          id: "mode-guide",
          className: "guide-list",
          textContent: "正在准备中文指引…",
        }),
      ]),
      createPageNode(document, "section", { className: "panel governance" }, [
        createPageNode(document, "div", { className: "panel-title", textContent: "聊天扩展" }),
        createPageNode(document, "div", {
          id: "world-summary",
          className: "governance-summary",
          textContent: "等待网关连接",
        }),
        createPageNode(document, "div", {
          id: "governance-status",
          className: "governance-status",
          textContent: "扩展状态：空闲",
        }),
        createPageNode(document, "ul", { id: "world-directory-list", className: "city-list compact-list" }),
        createPageNode(document, "ul", { id: "world-mirror-source-list", className: "city-list compact-list" }),
        createPageNode(document, "ul", { id: "world-square-list", className: "city-list compact-list" }),
        createPageNode(document, "ul", { id: "world-safety-list", className: "city-list compact-list" }),
        createPageNode(document, "ul", { id: "city-list", className: "city-list" }),
        createPageNode(document, "ul", { id: "resident-list", className: "city-list" }),
      ]),
      createPageNode(document, "section", { className: "panel rooms" }, [
        createPageNode(document, "div", { className: "panel-title", textContent: "会话列表" }),
        createPageNode(document, "ul", { id: "room-list", className: "room-list" }),
      ]),
    ]),
    createPageNode(document, "section", { className: "panel conversation" }, [
      createConversationStage(document),
      createConversationTools(document),
      createPageNode(document, "div", { id: "timeline", className: "timeline" }),
      createComposer(document),
    ]),
  ]);

  app.append(topbar, createCompactUserLoginCard(document), layout);
  body.appendChild(app);
}

function createCompactUserLoginCard(document) {
  return createPageNode(document, "section", {
    id: "resident-login-card",
    className: "wechat-login-card shell-hidden",
    attrs: { "aria-label": "居民登录" },
  }, [
    createPageNode(document, "div", { className: "wechat-login-copy" }, [
      createPageNode(document, "span", { className: "wechat-login-kicker", textContent: "居民身份" }),
      createPageNode(document, "strong", { id: "auth-status", textContent: "登录状态：访客模式" }),
      createPageNode(document, "span", { textContent: "连接网关后使用邮箱验证码登录；登录后可收发消息及使用完整功能。手机验证码与设备验证将在后续版本支持。" }),
    ]),
    createPageNode(document, "form", {
      id: "auth-request-form",
      className: "wechat-login-form",
      dataset: { authStep: "request" },
    }, [
      makeInput(document, "auth-resident-input", "text", "居民名/可选，新注册时使用"),
      makeSelect(document, "auth-delivery-select", [
        { value: "email", text: "邮箱验证码（已开通）", selected: true },
        { value: "mobile", text: "手机验证码（规划中）", disabled: true },
        { value: "device", text: "设备验证（规划中）", disabled: true },
      ], "email"),
      makeInput(document, "auth-email-input", "email", "接收验证码的邮箱"),
      makeInput(document, "auth-mobile-input", "tel", "手机号/可选反滥用"),
      makeInput(document, "auth-device-input", "text", "设备名/可选反滥用"),
      createButton(document, "auth-request-button", "登录 / 注册", "submit"),
    ]),
    createPageNode(document, "form", {
      id: "auth-verify-form",
      className: "wechat-login-form wechat-login-verify shell-hidden",
      dataset: { authStep: "verify" },
    }, [
      makeInput(document, "auth-challenge-input", "hidden", ""),
      createPageNode(document, "span", { className: "wechat-login-hint", textContent: "验证码来自上一步邮件" }),
      makeInput(document, "auth-code-input", "text", "输入邮箱验证码"),
      createButton(document, "auth-verify-button", "完成登录", "submit"),
    ]),
  ]);
}

function buildUserPage(document, options = {}) {
  const { omitStatusBadges = false, omitAuxPanels = false, omitIdentityRow = false } = options;
  const body = document.body;
  body.dataset.shellPage = "user";
  body.dataset.defaultShellMode = "user";
  body.dataset.workspace = "chat";

  const app = createPageNode(document, "div", { className: "app app-user-shell" });
  const topbar = createPageNode(document, "header", { className: "topbar" });
  const topbarMain = createPageNode(document, "div", { className: "topbar-main" });
  const masthead = createPageNode(document, "div", { className: "masthead" }, [
    createPageNode(document, "div", { id: "masthead-eyebrow", className: "eyebrow", textContent: "我和狗蛋儿的家 · 房间聊天" }),
    createPageNode(document, "h1", { id: "masthead-title", textContent: "像在房间里聊天一样继续说" }),
    createPageNode(document, "p", { id: "hero-note", className: "hero-note", textContent: "左边选会话，右边看消息，底部直接输入。用户页只保留聊天主路径。" }),
  ]);
  topbarMain.appendChild(masthead);
  topbar.appendChild(topbarMain);
  if (!omitStatusBadges) {
    topbar.appendChild(
      createPageNode(document, "div", { className: "status-cluster" }, [
        createPageNode(document, "span", { id: "shell-mode-badge", className: "badge", textContent: "入口：房间门牌" }),
        createPageNode(document, "span", { id: "transport-state", className: "badge", textContent: "连接：启动中" }),
        createPageNode(document, "span", { id: "storage-state", className: "badge", textContent: "历史：检测中" }),
        createPageNode(document, "span", { id: "gateway-state", className: "badge", textContent: "网关：未连接" }),
        createPageNode(document, "span", { id: "provider-state", className: "badge", textContent: "来源：未知" }),
        createPageNode(document, "span", { id: "world-state", className: "badge", textContent: "扩展：离线" }),
      ]),
    );
  }

  const layout = createPageNode(document, "main", {
    className: "layout layout-user-shell",
    attrs: { "aria-label": "用户聊天壳层" },
  });

  const roomsColumn = createPageNode(document, "aside", {
    className: "sidebar-stack sidebar-stack-user shell-column shell-column-rooms",
    dataset: { shellColumn: "rooms" },
  }, [
    createPageNode(document, "section", { className: "panel rooms rooms-user-shell" }, [
      createPageNode(document, "div", { className: "panel-shell-header" }, [
        createPageNode(document, "div", {}, [
          createPageNode(document, "div", { className: "panel-kicker", textContent: "会话栏" }),
          createPageNode(document, "div", { className: "panel-shell-title", textContent: "会话列表" }),
        ]),
        createPageNode(document, "p", {
          className: "panel-shell-note",
          textContent: "先选会话，再把注意力放回中间的 scene 和消息流。",
        }),
      ]),
      createPageNode(document, "ul", { id: "room-list", className: "room-list" }),
    ]),
  ]);

  const conversation = createPageNode(document, "section", {
    className: "panel conversation conversation-shell-user shell-column shell-column-scene",
    dataset: { shellColumn: "scene" },
    attrs: { "aria-label": "scene 区" },
  }, [
    createConversationStage(document),
    createPageNode(document, "div", { className: "conversation-stream conversation-stream-user" }, [
      createPageNode(document, "div", { className: "conversation-stream-header" }, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "消息流" }),
        createPageNode(document, "p", {
          className: "conversation-stream-note",
          textContent: "当前窗口里的对话会在这里展开，滚动和未读提示都保持可用。",
        }),
      ]),
      createPageNode(document, "div", { id: "timeline", className: "timeline" }),
    ]),
  ]);

  const detail = createPageNode(document, "aside", {
    className: "panel chat-detail conversation-shell-user shell-column shell-column-detail",
    dataset: { shellColumn: "detail" },
    attrs: { "aria-label": "角色与消息输入区" },
  }, [
    createPageNode(document, "div", { className: "panel-shell-header chat-detail-shell-header" }, [
      createPageNode(document, "div", {}, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "角色与消息输入区" }),
        createPageNode(document, "div", { className: "panel-shell-title", textContent: "房间资料" }),
      ]),
      createPageNode(document, "p", {
        className: "panel-shell-note",
        textContent: "这里复用 app.js 的资料面板钩子，同时把输入框留在右栏，方便持续开聊。",
      }),
    ]),
    createPageNode(document, "div", { className: "chat-detail-shell" }, [
      createPageNode(document, "div", { className: "chat-detail-summary" }, [
        createPageNode(document, "div", {
          id: "chat-detail-summary-title",
          className: "chat-detail-summary-title",
          textContent: "当前房间状态",
        }),
        createPageNode(document, "p", {
          id: "chat-detail-summary-copy",
          className: "chat-detail-summary-copy",
          textContent: "角色资料会随着会话切换更新，消息输入保持清楚可见。",
        }),
      ]),
      createPageNode(document, "section", {
        id: "chat-detail-card-shell",
        className: "chat-detail-card-shell",
        dataset: { roomVariant: "idle", roomMotif: "idle" },
        attrs: { "aria-label": "房间角色卡" },
      }, [
        createPageNode(document, "div", { className: "chat-detail-card-head" }, [
          createPageNode(document, "div", {
            id: "chat-detail-card-kicker",
            className: "panel-kicker",
            textContent: "角色卡",
          }),
          createPageNode(document, "div", {
            id: "chat-detail-card-title",
            className: "chat-detail-card-title",
            textContent: "当前房间角色卡",
          }),
        ]),
        createPageNode(document, "div", { className: "chat-detail-card-body" }, [
          createPageNode(document, "div", {
            id: "chat-detail-card-avatar",
            className: "chat-detail-card-avatar",
            dataset: { monogram: "房" },
            textContent: "房",
          }),
          createPageNode(document, "div", {
            id: "chat-detail-card-meta",
            className: "chat-detail-card-meta",
          }, [
            createPageNode(document, "div", { className: "chat-detail-card-meta-row" }, [
              createPageNode(document, "span", {
                className: "chat-detail-card-meta-label",
                textContent: "状态",
              }),
              createPageNode(document, "span", {
                className: "chat-detail-card-meta-value",
                textContent: "等待打开一个会话",
              }),
            ]),
          ]),
        ]),
        createPageNode(document, "div", {
          id: "chat-detail-card-actions",
          className: "chat-detail-card-actions",
          dataset: { roomVariant: "idle" },
        }),
      ]),
      createPageNode(document, "div", { id: "chat-detail-content", className: "chat-detail-content" }),
      createComposer(document, { omitIdentityRow }),
    ]),
  ]);

  layout.append(roomsColumn, conversation, detail);
  app.append(topbar, createCompactUserLoginCard(document), layout);
  body.appendChild(app);
}

function createNonUserRoomsPanel(document, shellPage) {
  const title = shellPage === "admin" ? "会话列表" : "会话列表";
  const note = shellPage === "admin"
    ? "居民、城主和值守会话统一排队，优先处理当前窗口里的事情。"
    : "群聊、私聊和系统会话统一入口，像产品会话列表而不是调试列表。";
  return createPageNode(document, "section", { className: "panel rooms" }, [
    createPageNode(document, "div", { className: "panel-shell-header" }, [
      createPageNode(document, "div", {}, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "会话队列" }),
        createPageNode(document, "div", { className: "panel-shell-title", textContent: title }),
      ]),
      createPageNode(document, "p", { className: "panel-shell-note", textContent: note }),
    ]),
    createPageNode(document, "div", { className: "panel-title", textContent: title }),
    createPageNode(document, "ul", { id: "room-list", className: "room-list" }),
  ]);
}

function createNonUserConversation(document, shellPage) {
  const conversationClass = shellPage === "admin"
    ? "panel conversation conversation-shell-admin"
    : "panel conversation";
  const panelTitle = shellPage === "admin" ? "当前消息" : "消息";
  const stageNote = shellPage === "admin"
    ? "先看消息，再做处理；动作都围着当前会话走。"
    : "先看消息，再看城市、公告、安全和身份分层。";
  const chips = shellPage === "admin"
    ? ["消息", "处理", "资料"]
    : ["会话", "消息", "资料"];
  const composerHint = shellPage === "admin"
    ? ["Enter 发送", "Shift+Enter 换行", "处理", "刷新"]
    : ["Enter 发送", "Shift+Enter 换行", "左侧切换会话"];
  const composerPlaceholder = shellPage === "admin" ? "先选会话，再写跟进或公告" : "先选会话，再输入消息";

  return createPageNode(document, "section", { className: conversationClass }, [
    createPageNode(document, "div", { className: "conversation-stage" }, [
      createPageNode(document, "div", { className: "conversation-stage-copy" }, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "聊天区" }),
        createPageNode(document, "div", { id: "room-stage-title", className: "conversation-stage-title", textContent: "当前会话" }),
        createPageNode(document, "p", { className: "conversation-stage-note", textContent: stageNote }),
      ]),
      createPageNode(document, "div", { className: "conversation-stage-side" }, chips.map((text) =>
        createPageNode(document, "span", { className: "stage-chip", textContent: text })
      )),
    ]),
    createPageNode(document, "div", { className: "panel-title", textContent: panelTitle }),
    createPageNode(document, "div", { id: "conversation-meta", className: "meta" }),
    createConversationTools(document),
    createPageNode(document, "div", { id: "timeline", className: "timeline" }),
    createPageNode(document, "form", { id: "composer", className: "composer" }, [
      createPageNode(document, "div", { className: "identity-row" }, [
        createPageNode(document, "label", { attrs: { for: "identity-input" }, textContent: "当前身份" }),
        makeInput(document, "identity-input", "text", "输入已分配的居民账号"),
      ]),
      createPageNode(document, "div", { className: "composer-row" }, [
        createPageNode(document, "textarea", {
          id: "composer-input",
          attrs: { rows: "1", autofocus: "" },
          textContent: "",
          placeholder: composerPlaceholder,
        }),
        createButton(document, "composer-send", "发送", "submit"),
      ]),
      createPageNode(
        document,
        "div",
        { className: `composer-hints ${shellPage === "admin" ? "composer-hints-admin" : "composer-hints-unified"}` },
        composerHint.map((text) => createPageNode(document, "span", { textContent: text })),
      ),
    ]),
  ]);
}

function createNonUserDetailPanel(document, shellPage) {
  return createPageNode(document, "aside", { className: "panel chat-detail" }, [
    createPageNode(document, "div", { className: "panel-shell-header panel-shell-header-detail" }, [
      createPageNode(document, "div", {}, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "资料" }),
        createPageNode(document, "div", {
          className: "panel-shell-title",
          textContent: shellPage === "admin" ? "会话详情" : "当前会话",
        }),
      ]),
      createPageNode(document, "p", {
        className: "panel-shell-note",
        textContent: shellPage === "admin"
          ? "对象信息、风险状态和会话上下文单独放右侧，别让右栏抢走消息流。"
          : "右侧持续显示上下文与动作位，但联合视图仍先像一页聊天会话。",
      }),
    ]),
    createPageNode(document, "div", {
      className: "panel-title",
      textContent: shellPage === "admin" ? "会话详情" : "当前会话",
    }),
    createPageNode(document, "div", { id: "chat-detail-content", className: "chat-detail-content" }),
  ]);
}

function buildNonUserPage(document, shellPage) {
  const body = document.body;
  body.dataset.shellPage = shellPage === "unified" ? "world-entry" : shellPage;
  body.dataset.defaultShellMode = shellPage;
  body.dataset.workspace = "chat";
  if (shellPage === "unified") {
    document.title = "我和狗蛋儿的家 · 世界入口";
  }

  const title = shellPage === "admin" ? "管理后台" : "城市外世界页";
  const mastheadTitle = shellPage === "admin"
    ? "左侧选工具，中间处理当前事务"
    : "房间在中间，治理顺着边走";
  const heroNote = shellPage === "admin"
    ? "后台按会话、居民、房间、安全、公告、世界和系统分组；日常先处理会话，高风险动作默认收起。"
    : "把主聊放在中间，这里就是城市外世界页；城市外壳按来源、城邦、公告、安全、身份顺序排开。";

  const app = createPageNode(document, "div", { className: "app" });
  const topbar = createPageNode(document, "header", { className: "topbar" });
  const topbarMain = createPageNode(document, "div", { className: "topbar-main" });
  const masthead = createPageNode(document, "div", { className: "masthead" }, [
    createPageNode(document, "div", {
      id: "masthead-eyebrow",
      className: "eyebrow",
      textContent: `我和狗蛋儿的家 · ${title}`,
    }),
    createPageNode(document, "h1", { id: "masthead-title", textContent: mastheadTitle }),
    createPageNode(document, "p", { id: "hero-note", className: "hero-note", textContent: heroNote }),
  ]);
  topbarMain.appendChild(masthead);
  topbar.appendChild(topbarMain);
  topbar.appendChild(
    createPageNode(document, "div", { className: "status-cluster" }, [
      createPageNode(document, "span", { id: "shell-mode-badge", className: "badge", textContent: `入口：${title}` }),
      createPageNode(document, "span", { id: "transport-state", className: "badge", textContent: "连接：探测中" }),
      createPageNode(document, "span", { id: "storage-state", className: "badge", textContent: "历史：准备中" }),
      createPageNode(document, "span", { id: "gateway-state", className: "badge", textContent: "消息源：待定" }),
      createPageNode(document, "span", { id: "provider-state", className: "badge", textContent: "身份：待定" }),
      createPageNode(document, "span", {
        id: "world-state",
        className: "badge",
        textContent: shellPage === "admin" ? "城邦：治理中" : "世界：展开",
      }),
    ]),
  );

  const layout = createPageNode(document, "main", {
    className: shellPage === "admin" ? "layout layout-admin-shell" : "layout layout-unified-shell",
  });

  const sidebar = createPageNode(document, "aside", {
    className: shellPage === "admin" ? "sidebar-stack sidebar-stack-admin" : "sidebar-stack",
  });
  sidebar.appendChild(createNonUserRoomsPanel(document, shellPage));
  sidebar.appendChild(createUserGovernancePanel(document));
  sidebar.appendChild(createUserAuthPanel(document));

  layout.appendChild(sidebar);
  layout.appendChild(createNonUserConversation(document, shellPage));
  layout.appendChild(createNonUserDetailPanel(document, shellPage));
  app.append(topbar, layout);
  body.appendChild(app);

  if (shellPage === "unified") {
    const routePanel = createPageNode(document, "div", { id: "world-routes", className: "world-route-panel" });
    const routeCard = createPageNode(document, "div", { className: "world-route-card" });
    const routeList = createPageNode(document, "div", { className: "world-route-list" }, [
      createPageNode(document, "a", { className: "world-route-option world-route-option-square", attrs: { href: "./world-square.html" } }, [
        createPageNode(document, "strong", { textContent: "世界广场" }),
        createPageNode(document, "span", { textContent: "打开之前绘制的世界广场完整素材，作为公共广场入口。" }),
        createPageNode(document, "span", { className: "world-route-status", textContent: "概念图 · 公共广场" }),
      ]),
      createPageNode(document, "a", { className: "world-route-option", attrs: { href: "./index.html" } }, [
        createPageNode(document, "strong", { textContent: "凛冬城主城" }),
        createPageNode(document, "span", { textContent: "当前主城广场，群聊与公告入口。" }),
      ]),
      createPageNode(document, "a", { className: "world-route-option", attrs: { href: "./index.html?city=harbor" } }, [
        createPageNode(document, "strong", { textContent: "海港主城" }),
        createPageNode(document, "span", { textContent: "后续外城入口，暂接入同一主城壳。" }),
      ]),
      createPageNode(document, "a", { className: "world-route-option", attrs: { href: "./index.html?city=mountain" } }, [
        createPageNode(document, "strong", { textContent: "山城主城" }),
        createPageNode(document, "span", { textContent: "预留线路，用于后续多城邦扩展。" }),
      ]),
    ]);
    routeCard.appendChild(routeList);
    routePanel.appendChild(routeCard);
    body.appendChild(routePanel);
  }
}

function createSectionPanel(document, className, title, note) {
  return createPageNode(document, "section", { className: `panel ${className}` }, [
    createPageNode(document, "div", { className: "panel-title", textContent: title }),
    createPageNode(document, "div", { className: "panel-shell-header" }, [
      createPageNode(document, "div", {}, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: className.replace("-panel", "") }),
        createPageNode(document, "div", { className: "panel-shell-title", textContent: title }),
      ]),
      createPageNode(document, "p", { className: "panel-shell-note", textContent: note }),
    ]),
  ]);
}

function createUserGovernancePanel(document) {
  const panel = createPageNode(document, "section", { className: "panel governance" }, [
    createPageNode(document, "div", { className: "panel-title", textContent: "边缘抽屉" }),
    createPageNode(document, "div", { id: "world-summary", className: "governance-summary", textContent: "聊天优先。房间主聊在前，世界外壳收在抽屉里。" }),
    createPageNode(document, "div", { id: "governance-status", className: "governance-status", textContent: "边缘抽屉：收起" }),
  ]);
  const details = createPageNode(document, "details", { className: "governance-details" }, [
    createPageNode(document, "summary", { textContent: "展开边缘抽屉" }),
    createPageNode(document, "div", { className: "governance-forms" }, [
      createPageNode(document, "div", { id: "mode-guide", className: "guide-list compact-guide-list", textContent: "正在准备中文指引…" }),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "世界目录" }), createPageNode(document, "ul", { id: "world-directory-list", className: "city-list compact-list" })]),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "镜像源" }), createPageNode(document, "ul", { id: "world-mirror-source-list", className: "city-list compact-list" })]),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "世界广场" }), createPageNode(document, "ul", { id: "world-square-list", className: "city-list compact-list" })]),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "世界安全" }), createPageNode(document, "ul", { id: "world-safety-list", className: "city-list compact-list" })]),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "城市列表" }), createPageNode(document, "ul", { id: "city-list", className: "city-list" })]),
      createPageNode(document, "div", { className: "governance-block" }, [createPageNode(document, "div", { className: "section-label", textContent: "居民目录" }), createPageNode(document, "ul", { id: "resident-list", className: "city-list" })]),
      createForm(document, "city-create-form", "创建城市", [
        makeInput(document, "city-title-input", "text", "城市名称"),
        makeInput(document, "city-slug-input", "text", "城市标识（可选）"),
        createPageNode(document, "textarea", { id: "city-description-input" }),
        createButton(document, "city-create-button", "创建城市"),
      ], true),
      createForm(document, "city-join-form", "加入城市", [
        makeInput(document, "city-join-input", "text", "城市标识或城市 ID"),
        createButton(document, "city-join-button", "加入城市"),
      ]),
      createForm(document, "room-create-form", "创建公共房间", [
        makeInput(document, "room-city-input", "text", "城市标识或城市 ID"),
        makeInput(document, "room-title-input", "text", "房间名称"),
        makeInput(document, "room-slug-input", "text", "房间标识（可选）"),
        createPageNode(document, "textarea", { id: "room-description-input" }),
        createButton(document, "room-create-button", "创建房间"),
      ], true),
      createForm(document, "direct-open-form", "发起私聊", [
        makeInput(document, "direct-peer-input", "text", "居民 ID"),
        createButton(document, "direct-open-button", "打开私聊"),
      ]),
      createForm(document, "world-mirror-form", "添加镜像源", [
        makeInput(document, "world-mirror-url-input", "url", "https://seed-city.example.com"),
        createButton(document, "world-mirror-button", "添加镜像"),
      ], true),
      createForm(document, "world-notice-form", "发布世界公告", [
        makeInput(document, "world-notice-title-input", "text", "公告标题"),
        createPageNode(document, "select", { id: "world-notice-severity-select" }),
        makeInput(document, "world-notice-tags-input", "text", "标签，使用逗号分隔"),
        createPageNode(document, "textarea", { id: "world-notice-body-input" }),
        createButton(document, "world-notice-button", "发布公告"),
      ], true),
      createForm(document, "world-trust-form", "更新城市信任状态", [
        makeInput(document, "world-trust-city-input", "text", "城市标识或城市 ID"),
        createPageNode(document, "select", { id: "world-trust-state-select" }),
        createPageNode(document, "textarea", { id: "world-trust-reason-input" }),
        createButton(document, "world-trust-button", "更新状态"),
      ], true),
      createForm(document, "world-advisory-form", "发布安全通告", [
        createPageNode(document, "select", { id: "world-advisory-subject-kind-select" }),
        makeInput(document, "world-advisory-subject-input", "text", "安全通告对象"),
        makeInput(document, "world-advisory-action-input", "text", "建议动作"),
        createPageNode(document, "textarea", { id: "world-advisory-reason-input" }),
        createButton(document, "world-advisory-button", "发布通告"),
      ], true),
      createForm(document, "world-report-review-form", "审查举报", [
        makeInput(document, "world-report-review-id-input", "text", "举报编号"),
        createPageNode(document, "select", { id: "world-report-review-status-select" }),
        createPageNode(document, "select", { id: "world-report-review-city-state-select" }),
        createPageNode(document, "textarea", { id: "world-report-review-resolution-input" }),
        createButton(document, "world-report-review-button", "审查举报"),
      ], true),
      createForm(document, "world-report-form", "提交举报", [
        makeInput(document, "world-report-city-input", "text", "城市标识或城市 ID"),
        createPageNode(document, "select", { id: "world-report-target-kind-select" }),
        makeInput(document, "world-report-target-input", "text", "举报对象"),
        createPageNode(document, "textarea", { id: "world-report-summary-input" }),
        createPageNode(document, "textarea", { id: "world-report-evidence-input" }),
        createButton(document, "world-report-button", "提交举报"),
      ]),
      createForm(document, "world-resident-sanction-form", "发布居民制裁", [
        makeInput(document, "world-resident-id-input", "text", "居民 ID"),
        makeInput(document, "world-resident-city-input", "text", "城市标识或城市 ID"),
        makeInput(document, "world-resident-email-input", "email", "邮箱"),
        makeInput(document, "world-resident-mobile-input", "text", "手机号"),
        makeInput(document, "world-resident-device-input", "text", "设备物理地址"),
        createPageNode(document, "textarea", { id: "world-resident-reason-input" }),
        createButton(document, "world-resident-sanction-button", "发布制裁"),
      ], true),
    ]),
  ]);
  panel.appendChild(details);
  return panel;
}

function createProviderActions(document) {
  const wrap = createPageNode(document, "div", { className: "provider-actions" }, [
    createButton(document, "provider-connect-button", "连接消息来源", "submit"),
    createButton(document, "provider-disconnect-button", "断开连接", "button", "secondary"),
  ]);
  return wrap;
}

function createForm(document, id, label, children, admin = false) {
  const form = createPageNode(document, "form", {
    id,
    className: "inline-form compact-form",
    dataset: admin ? { shellRole: "admin" } : undefined,
  });
  form.appendChild(createPageNode(document, "div", { className: "section-label", textContent: label }));
  for (const child of children) form.appendChild(child);
  return form;
}

function createButton(document, id, text, type = "button", extraClass = "") {
  return createPageNode(document, "button", {
    id,
    className: extraClass ? extraClass : undefined,
    attrs: { type },
    textContent: text,
  });
}

function createUserAuthPanel(document) {
  const panel = createPageNode(document, "section", { className: "panel auth" }, [
    createPageNode(document, "div", { className: "panel-title", textContent: "房间身份" }),
    createPageNode(document, "div", { className: "panel-shell-header" }, [
      createPageNode(document, "div", {}, [
        createPageNode(document, "div", { className: "panel-kicker", textContent: "身份" }),
        createPageNode(document, "div", { className: "panel-shell-title", textContent: "房间身份" }),
      ]),
      createPageNode(document, "p", { className: "panel-shell-note", textContent: "身份和验证码只在需要时出现，平时不打扰聊天。" }),
    ]),
    createPageNode(document, "div", { id: "auth-status", className: "governance-status", textContent: "认证状态：空闲" }),
  ]);
  const details = createPageNode(document, "details", { className: "auth-details" }, [
    createPageNode(document, "summary", { textContent: "身份与验证码" }),
    createPageNode(document, "div", { className: "auth-forms" }, [
      createForm(document, "auth-request-form", "邮箱验证码登录", [
        makeInput(document, "auth-email-input", "email", "邮箱地址"),
        makeInput(document, "auth-mobile-input", "text", "手机号（反滥用句柄）"),
        makeInput(document, "auth-device-input", "text", "设备物理地址（可选，如网卡 MAC）"),
        createButton(document, "auth-request-button", "申请验证码", "submit"),
      ]),
      createForm(document, "auth-verify-form", "验证验证码", [
        makeInput(document, "auth-challenge-input", "text", "挑战编号"),
        makeInput(document, "auth-code-input", "text", "6 位验证码"),
        createButton(document, "auth-verify-button", "完成登录", "submit"),
      ]),
    ]),
  ]);
  panel.appendChild(details);
  return panel;
}

function createConversationStage(document) {
  return createPageNode(document, "div", { className: "conversation-stage user-stage" }, [
    createPageNode(document, "div", { className: "scene-hotspots creative-hotspots" }),
    createPageNode(document, "div", { className: "conversation-stage-copy" }, [
      createPageNode(document, "div", { className: "panel-kicker", textContent: "聊天场景" }),
      createPageNode(document, "div", { id: "room-stage-title", className: "conversation-stage-title", textContent: "当前会话" }),
      createPageNode(document, "div", { id: "conversation-meta", className: "meta conversation-meta" }),
      createPageNode(document, "div", { className: "conversation-stage-canvas-wrap" }, [
        createPageNode(document, "canvas", {
          id: "room-stage-canvas",
          className: "conversation-stage-canvas",
          attrs: { "aria-label": "房间场景文字画布" },
        }),
      ]),
      createPageNode(document, "p", { className: "conversation-stage-note", textContent: "消息和输入贴着屋里走，先看聊天，再看别的。" }),
    ]),
    createPageNode(document, "div", { className: "conversation-stage-side", attrs: { "aria-label": "房间角色资料" } }, [
      createPageNode(document, "div", { className: "conversation-stage-canvas-wrap" }, [
        createPageNode(document, "canvas", {
          id: "room-stage-portrait-canvas",
          className: "conversation-stage-canvas",
          attrs: { "aria-label": "房间角色资料画布" },
        }),
      ]),
    ]),
  ]);
}

function createConversationTools(document) {
  return createPageNode(document, "div", { className: "conversation-tools" }, [
    createPageNode(document, "select", { id: "export-format-select", attrs: { "aria-label": "导出格式" } }),
    createButton(document, "export-current-button", "导出当前会话", "button", "secondary"),
    createButton(document, "export-all-button", "导出全部历史", "button", "secondary"),
  ]);
}

function createComposer(document, options = {}) {
  const { omitIdentityRow = false } = options;
  const children = [];
  if (!omitIdentityRow) {
    children.push(
      createPageNode(document, "div", { className: "identity-row" }, [
        createPageNode(document, "label", { attrs: { for: "identity-input" }, textContent: "当前身份" }),
        makeInput(document, "identity-input", "text", "输入已分配的居民账号"),
      ]),
    );
  }
  children.push(
    createPageNode(document, "div", { className: "composer-row" }, [
      createPageNode(document, "textarea", { id: "composer-input", attrs: { rows: "1", autofocus: "" }, textContent: "", }),
      createButton(document, "composer-send", "发送", "submit"),
    ]),
    createPageNode(document, "div", { className: "composer-hints composer-hints-user" }, [
      createPageNode(document, "span", { textContent: "Enter 发送" }),
      createPageNode(document, "span", { textContent: "Shift+Enter 换行" }),
      createPageNode(document, "span", { textContent: "↑ 取回上一条" }),
      createPageNode(document, "span", { textContent: "先选会话再聊" }),
    ]),
  );
  const composer = createPageNode(document, "form", { id: "composer", className: "composer" }, children);
  return composer;
}

async function loadShellApp(shellPage, options = {}) {
  const {
    useGeneratedFixtures = false,
    generatedShellFixture = "generated/state.json",
    localStorageEntries = {},
    locationSearch = "",
    gatewayBaseUrl = "",
    exportResponse = null,
    gatewayMessageShouldFail = false,
    gatewayMessageFailuresBeforeSuccess = 0,
    gatewayShellStateShouldFail = false,
    gatewayShellStateFailureStatus = 503,
    gatewayShellStateFixture = null,
    gatewayProviderState = null,
  } = options;
  const previous = captureGlobals();
  const document = new FakeDocument();
  const activeTimeouts = new Set();
  const activeIntervals = new Set();
  const trackedSetTimeout = (callback, delay = 0, ...args) => {
    const id = previous.setTimeout(() => {
      activeTimeouts.delete(id);
      callback(...args);
    }, delay);
    activeTimeouts.add(id);
    return id;
  };
  const trackedClearTimeout = (id) => {
    activeTimeouts.delete(id);
    return previous.clearTimeout(id);
  };
  const trackedSetInterval = (callback, delay = 0, ...args) => {
    const id = previous.setInterval(callback, delay, ...args);
    activeIntervals.add(id);
    return id;
  };
  const trackedClearInterval = (id) => {
    activeIntervals.delete(id);
    return previous.clearInterval(id);
  };
  const window = {
    document,
    location: {
      href: new URL(`../${shellPage}.html${locationSearch}`, import.meta.url).href,
      protocol: "file:",
      origin: "null",
    },
    localStorage: createLocalStorage(),
    matchMedia: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }),
    addEventListener() {},
    removeEventListener() {},
    setTimeout: trackedSetTimeout,
    clearTimeout: trackedClearTimeout,
    setInterval: trackedSetInterval,
    clearInterval: trackedClearInterval,
    requestAnimationFrame: (callback) => trackedSetTimeout(callback, 0),
    cancelAnimationFrame: (id) => trackedClearTimeout(id),
  };

  if (shellPage === "hub") {
    buildHubPage(document);
  } else if (shellPage === "user") {
    buildUserPage(document, options);
  } else {
    buildNonUserPage(document, shellPage);
  }
  document.activeElement = document.body;

  for (const [key, value] of Object.entries(localStorageEntries)) {
    window.localStorage.setItem(key, value);
  }
  const fetchCalls = [];
  const fetchRequests = [];
  const eventSourceCalls = [];
  const eventSources = new Set();
  let gatewayMessageFailureCount = 0;
  let shellStateFixture = null;

  async function readGatewayShellState() {
    if (!shellStateFixture) {
      shellStateFixture = gatewayShellStateFixture
        ? structuredClone(gatewayShellStateFixture)
        : await readJsonFixture(generatedShellFixture);
      // generated/state.json is a legacy local preview snapshot. When the
      // fixture is served as a fake Gateway response, fill the cursor that
      // the real /v1/shell/state contract always includes.
      if (typeof shellStateFixture?.state_version !== "string") {
        shellStateFixture.state_version = "shell:v1:test-fixture";
      }
    }
    return shellStateFixture;
  }

  function appendGatewayMessage(payload = {}) {
    if (!shellStateFixture) return null;
    const roomId = typeof payload.room_id === "string" ? payload.room_id : "";
    const sender = typeof payload.sender === "string" ? payload.sender : "";
    const text = typeof payload.text === "string" ? payload.text : "";
    const quickAction = typeof payload.quick_action === "string" ? payload.quick_action : "";
    if (!roomId || !sender || !text) return null;
    const timestampMs = Date.now();
    const messageId = `msg:test-shell-message:${timestampMs}`;
    const message = {
      message_id: messageId,
      id: messageId,
      sender,
      timestamp_ms: timestampMs,
      timestamp_label: "刚刚",
      timestamp: "刚刚",
      text,
      delivery_status: "delivered",
    };
    if (quickAction) {
      message.quick_action = quickAction;
    }
    const conversations = shellStateFixture?.conversation_shell?.conversations;
    const conversation = Array.isArray(conversations)
      ? conversations.find((item) => item?.conversation_id === roomId)
      : null;
    if (conversation) {
      conversation.messages = Array.isArray(conversation.messages) ? conversation.messages : [];
      conversation.messages.push(message);
      shellStateFixture.conversation_shell.active_conversation_id = roomId;
    }
    const rooms = shellStateFixture?.rooms;
    const room = Array.isArray(rooms) ? rooms.find((item) => item?.id === roomId) : null;
    if (room) {
      room.messages = Array.isArray(room.messages) ? room.messages : [];
      room.messages.push(message);
      shellStateFixture.active_room_id = roomId;
    }
    shellStateFixture.state_version = `shell:v1:${timestampMs}`;
    return message;
  }

  function findGatewayMessage(roomId, messageId) {
    if (!shellStateFixture || !roomId || !messageId) return null;
    const conversations = shellStateFixture?.conversation_shell?.conversations;
    const conversation = Array.isArray(conversations)
      ? conversations.find((item) => item?.conversation_id === roomId)
      : null;
    const conversationMessage = Array.isArray(conversation?.messages)
      ? conversation.messages.find((item) => item?.message_id === messageId || item?.id === messageId)
      : null;
    const rooms = shellStateFixture?.rooms;
    const room = Array.isArray(rooms) ? rooms.find((item) => item?.id === roomId) : null;
    const roomMessage = Array.isArray(room?.messages)
      ? room.messages.find((item) => item?.message_id === messageId || item?.id === messageId)
      : null;
    return { conversationMessage, roomMessage };
  }

  function editGatewayMessage(payload = {}) {
    const roomId = typeof payload.room_id === "string" ? payload.room_id : "";
    const messageId = typeof payload.message_id === "string" ? payload.message_id : "";
    const actor = typeof payload.actor === "string" ? payload.actor : "";
    const text = typeof payload.text === "string" ? payload.text.trim() : "";
    const found = findGatewayMessage(roomId, messageId);
    const targets = [found?.conversationMessage, found?.roomMessage].filter(Boolean);
    if (!targets.length || !actor || !text) return null;
    const timestampMs = Date.now();
    for (const message of targets) {
      message.text = text;
      message.is_edited = true;
      message.edited_by = actor;
      message.edited_at_ms = timestampMs;
    }
    shellStateFixture.state_version = `shell:v1:${timestampMs}`;
    return { messageId, timestampMs, text, actor };
  }

  function recallGatewayMessage(payload = {}) {
    const roomId = typeof payload.room_id === "string" ? payload.room_id : "";
    const messageId = typeof payload.message_id === "string" ? payload.message_id : "";
    const actor = typeof payload.actor === "string" ? payload.actor : "";
    const found = findGatewayMessage(roomId, messageId);
    const targets = [found?.conversationMessage, found?.roomMessage].filter(Boolean);
    if (!targets.length || !actor) return null;
    const timestampMs = Date.now();
    for (const message of targets) {
      message.text = "消息已撤回";
      message.is_recalled = true;
      message.recalled_by = actor;
      message.recalled_at_ms = timestampMs;
    }
    shellStateFixture.state_version = `shell:v1:${timestampMs}`;
    return { messageId, timestampMs, actor };
  }

  function openGatewayDirectConversation(payload = {}) {
    if (!shellStateFixture) return null;
    const requester = typeof payload.requester_id === "string" ? payload.requester_id.trim() : "";
    const peer = typeof payload.peer_id === "string" ? payload.peer_id.trim() : "";
    if (!requester || !peer) return null;
    const conversationId = `dm:${[requester, peer].sort().join(":")}`;
    const title = `正在与 ${peer} 聊天`;
    const conversations = shellStateFixture.conversation_shell?.conversations;
    if (Array.isArray(conversations) && !conversations.some((item) => item?.conversation_id === conversationId)) {
      conversations.push({
        conversation_id: conversationId,
        title,
        subtitle: "私聊",
        meta: "",
        kind_hint: "direct",
        participant_label: peer,
        route_label: "居所直聊",
        list_summary: `${peer} · 私聊已就绪`,
        status_line: "私聊正常",
        thread_headline: title,
        chat_status_summary: "私聊正常",
        queue_summary: null,
        preview_text: null,
        last_activity_label: "刚刚",
        activity_time_label: "刚刚",
        overview_summary: `与 ${peer} 的私聊`,
        context_summary: "点对点会话",
        member_count: 2,
        caretaker: null,
        detail_card: null,
        workflow: null,
        inline_actions: [],
        messages: [],
      });
    }
    if (shellStateFixture.conversation_shell) {
      shellStateFixture.conversation_shell.active_conversation_id = conversationId;
    }
    shellStateFixture.scene_render = shellStateFixture.scene_render || {};
    shellStateFixture.scene_render.scenes = Array.isArray(shellStateFixture.scene_render.scenes)
      ? shellStateFixture.scene_render.scenes
      : [];
    if (!shellStateFixture.scene_render.scenes.some((item) => item?.conversation_id === conversationId)) {
      shellStateFixture.scene_render.scenes.push({
        conversation_id: conversationId,
        scene_banner: null,
        scene_summary: `与 ${peer} 的居所私聊`,
        room_variant: "home",
        room_motif: "residence",
        stage: null,
        portrait: null,
      });
    }
    shellStateFixture.state_version = `shell:v1:${Date.now()}`;
    return conversationId;
  }

  class FakeEventSource {
    constructor(url) {
      this.url = String(url);
      this.readyState = 0;
      this.listeners = new Map();
      this.onerror = null;
      eventSourceCalls.push(this.url);
      eventSources.add(this);
    }

    addEventListener(type, handler) {
      if (!this.listeners.has(type)) {
        this.listeners.set(type, []);
      }
      this.listeners.get(type).push(handler);
    }

    close() {
      this.readyState = 2;
      eventSources.delete(this);
    }

    emit(type, data) {
      const event = { data: typeof data === "string" ? data : JSON.stringify(data) };
      for (const handler of this.listeners.get(type) || []) {
        handler(event);
      }
    }
  }

  globalThis.window = window;
  globalThis.document = document;
  globalThis.HTMLElement = FakeElement;
  globalThis.Element = FakeElement;
  globalThis.Event = FakeEvent;
  globalThis.CustomEvent = FakeEvent;
  Object.defineProperty(globalThis, "navigator", {
    value: {
      language: "zh-CN",
      userAgent: "FakeDOM/1.0",
      vendor: "",
    },
    configurable: true,
    writable: true,
  });
  globalThis.localStorage = window.localStorage;
  globalThis.setTimeout = trackedSetTimeout;
  globalThis.clearTimeout = trackedClearTimeout;
  globalThis.setInterval = trackedSetInterval;
  globalThis.clearInterval = trackedClearInterval;
  globalThis.requestAnimationFrame = window.requestAnimationFrame;
  globalThis.cancelAnimationFrame = window.cancelAnimationFrame;
  globalThis.EventSource = FakeEventSource;
  globalThis.fetch = async (url, init = {}) => {
    fetchCalls.push(String(url));
    fetchRequests.push({ url: String(url), init });
    if (gatewayBaseUrl && typeof url === "string" && url.startsWith(gatewayBaseUrl)) {
      if (url === `${gatewayBaseUrl}/v1/shell/bootstrap`) {
        return responseFromJson(
          await readJsonFixture(useGeneratedFixtures ? "generated/bootstrap.json" : "bootstrap.sample.json"),
        );
      }
      if (url === `${gatewayBaseUrl}/v1/shell/state` || url.startsWith(`${gatewayBaseUrl}/v1/shell/state?`)) {
        if (gatewayShellStateShouldFail) {
          return responseFromJson(
            { Error: { message: "模拟 Gateway shell state 失败" } },
            { status: gatewayShellStateFailureStatus },
          );
        }
        return responseFromJson(await readGatewayShellState());
      }
      if (url === `${gatewayBaseUrl}/v1/shell/message`) {
        await readGatewayShellState();
        let payload = {};
        try {
          payload = typeof init?.body === "string" ? JSON.parse(init.body) : {};
        } catch {
          payload = {};
        }
        if (
          gatewayMessageShouldFail ||
          gatewayMessageFailureCount < gatewayMessageFailuresBeforeSuccess
        ) {
          gatewayMessageFailureCount += 1;
          return responseFromJson(
            { Error: { message: "模拟发送失败" } },
            { status: 400 },
          );
        }
        const message = appendGatewayMessage(payload);
        return responseFromJson({
          ok: true,
          conversation_id: payload?.room_id || "room:city:core-harbor:lobby",
          message_id: message?.message_id || "msg:test-shell-message",
          delivery_status: "delivered",
          delivered_at_ms: message?.timestamp_ms || Date.now(),
        });
      }
      if (url === `${gatewayBaseUrl}/v1/shell/message/edit`) {
        await readGatewayShellState();
        let payload = {};
        try {
          payload = typeof init?.body === "string" ? JSON.parse(init.body) : {};
        } catch {
          payload = {};
        }
        const edited = editGatewayMessage(payload);
        if (!edited) {
          return responseFromJson(
            { Error: { message: "模拟编辑失败" } },
            { status: 400 },
          );
        }
        return responseFromJson({
          ok: true,
          conversation_id: payload.room_id,
          message_id: edited.messageId,
          edit_status: "edited",
          edited_at_ms: edited.timestampMs,
          edited_by: edited.actor,
          text: edited.text,
        });
      }
      if (url === `${gatewayBaseUrl}/v1/shell/message/recall`) {
        await readGatewayShellState();
        let payload = {};
        try {
          payload = typeof init?.body === "string" ? JSON.parse(init.body) : {};
        } catch {
          payload = {};
        }
        const recalled = recallGatewayMessage(payload);
        if (!recalled) {
          return responseFromJson(
            { Error: { message: "模拟撤回失败" } },
            { status: 400 },
          );
        }
        return responseFromJson({
          ok: true,
          conversation_id: payload.room_id,
          message_id: recalled.messageId,
          recall_status: "recalled",
          recalled_at_ms: recalled.timestampMs,
          recalled_by: recalled.actor,
        });
      }
      if (url === `${gatewayBaseUrl}/v1/direct/open`) {
        await readGatewayShellState();
        let payload = {};
        try {
          payload = typeof init?.body === "string" ? JSON.parse(init.body) : {};
        } catch {
          payload = {};
        }
        const conversationId = openGatewayDirectConversation(payload);
        return responseFromJson({
          conversation_id: conversationId || "dm:rsaga:qa-peer",
          kind: "direct",
          scope: "private",
          members: [payload.requester_id, payload.peer_id].filter(Boolean),
        });
      }
      if (url === `${gatewayBaseUrl}/v1/auth/preflight`) {
        return responseFromJson({ allowed: true, normalized_email: "rsaga@example.com", blocked_reasons: [] });
      }
      if (url === `${gatewayBaseUrl}/v1/auth/email-otp/request`) {
        return responseFromJson({
          challenge_id: "otp:test",
          masked_email: "r***@example.com",
          expires_at_ms: Date.now() + 300000,
          delivery_mode: "dev-inline",
          dev_code: "123456",
        });
      }
      if (url === `${gatewayBaseUrl}/v1/auth/email-otp/verify`) {
        return responseFromJson({
          resident_id: "rsaga",
          email_masked: "r***@example.com",
          token_type: "Bearer",
          session_token: "lbst_test_session_token",
          session: {
            session_id: "session:test",
            resident_id: "rsaga",
            issued_at_ms: Date.now(),
            expires_at_ms: Date.now() + 300000,
          },
        });
      }
      if (url === `${gatewayBaseUrl}/v1/provider`) {
        return responseFromJson(
          gatewayProviderState || {
            mode: "cloudflare",
            reachable: true,
            connection_state: "connected",
            base_url: "https://cloudflare.com/fake-provider",
          },
        );
      }
      if (url.startsWith(`${gatewayBaseUrl}/v1/export?`)) {
        const response = exportResponse || {
          status: 200,
          body: {
            content: "# 我和狗蛋儿的家导出\n\n测试导出内容",
          },
        };
        return responseFromJson(response.body, { status: response.status || 200 });
      }
      if (url === `${gatewayBaseUrl}/v1/world-entry` && !gatewayBaseUrl.includes("59999")) {
        return responseFromJson({
          title: "世界入口",
          station_label: "地铁候车站",
          current_city_slug: "core-harbor",
          source_summary: "2 条线路 · 2 个镜像 · 1 条公告 · 0 条安全提示",
          route_count: 2,
          mirror_count: 2,
          notice_count: 1,
          advisory_count: 0,
          routes: [
            {
              city_id: "city:core-harbor",
              slug: "core-harbor",
              title: "核心港",
              description: "当前主城广场，群聊与公告入口。",
              href: "./index.html",
              trust_state: "Healthy",
              status_label: "健康 · 可镜像",
              mirror_enabled: true,
              resident_count: 1,
              public_room_count: 1,
              source_kind: "local",
              is_current: true,
            },
            {
              city_id: "city:signal-bay",
              slug: "signal-bay",
              title: "Signal Bay",
              description: "A visible city route for the world entry station",
              href: "./index.html?city=signal-bay",
              trust_state: "Healthy",
              status_label: "健康 · 可镜像",
              mirror_enabled: true,
              resident_count: 0,
              public_room_count: 1,
              source_kind: "local",
              is_current: false,
            },
          ],
        });
      }
      return responseNotFound();
    }
    if (url === "./generated/bootstrap.json") {
      return responseFromJson(
        await readJsonFixture(useGeneratedFixtures ? "generated/bootstrap.json" : "bootstrap.sample.json"),
      );
    }
    if (url === "./bootstrap.sample.json") {
      return responseFromJson(await readJsonFixture("bootstrap.sample.json"));
    }
    if (url === "./generated/state.json") {
      if (useGeneratedFixtures) {
        return responseFromJson(await readJsonFixture(generatedShellFixture));
      }
      return responseNotFound();
    }
    return responseNotFound();
  };

  const appPath = fileURLToPath(new URL("../app.js", import.meta.url));
  const appSource = await fs.readFile(appPath, "utf8");
  const transformedSource = [
    "const window = globalThis.window;",
    "const document = globalThis.document;",
    "const HTMLElement = globalThis.HTMLElement;",
    "const Element = globalThis.Element;",
    "const Event = globalThis.Event;",
    "const CustomEvent = globalThis.CustomEvent;",
    "const EventSource = globalThis.EventSource;",
    "const navigator = globalThis.navigator;",
    "const localStorage = globalThis.localStorage;",
    "const requestAnimationFrame = globalThis.requestAnimationFrame;",
    "const cancelAnimationFrame = globalThis.cancelAnimationFrame;",
    "const fetch = globalThis.fetch;",
    rewriteAppLocalImports(appSource),
  ].join("\n");

  const tempPath = path.join(
    os.tmpdir(),
    `lobster-web-shell-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}.mjs`,
  );
  await fs.writeFile(tempPath, transformedSource, "utf8");
  try {
    await import(`${pathToFileURL(tempPath).href}?case=${Date.now()}-${Math.random().toString(16).slice(2)}`);
  } finally {
    await fs.unlink(tempPath).catch(() => {});
  }
  await waitFor(() => {
    if (shellPage === "hub") {
      return (
        Boolean(document.querySelector("#masthead-title")?.textContent) &&
        Boolean(document.querySelector("#shell-mode-badge")?.textContent)
      );
    }
    const composer = document.querySelector("#composer-input");
    const activeRoom = document.querySelector(".room-button.active");
    const allowsLoginBlockedComposer =
      shellPage === "user" && Boolean(gatewayBaseUrl) && !localStorageEntries["lobster-identity"];
    const providerConnectionState = String(gatewayProviderState?.connection_state || "").toLowerCase();
    const allowsOfflineBlockedComposer =
      Boolean(gatewayBaseUrl) &&
      Boolean(
        gatewayProviderState?.reachable === false ||
        providerConnectionState === "disconnected" ||
        providerConnectionState === "offline" ||
        gatewayShellStateShouldFail,
      );
    const allowsGatewayShellFailure = Boolean(gatewayBaseUrl && gatewayShellStateShouldFail);
    return (
      document.body?.dataset?.shellMode === shellPage &&
      (Boolean(activeRoom) || allowsGatewayShellFailure) &&
      Boolean(composer) &&
      (composer.disabled === false || allowsLoginBlockedComposer || allowsOfflineBlockedComposer) &&
      typeof composer.placeholder === "string" &&
      composer.placeholder.length > 0
    );
  });
  if (shellPage === "hub") {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  return {
    document,
    window,
    fetchCalls,
    fetchRequests,
    eventSourceCalls,
    emitEventSource(type, data) {
      for (const eventSource of Array.from(eventSources)) {
        eventSource.emit(type, data);
      }
    },
    cleanup() {
      for (const eventSource of eventSources) {
        eventSource.close();
      }
      for (const id of activeIntervals) {
        previous.clearInterval(id);
      }
      for (const id of activeTimeouts) {
        previous.clearTimeout(id);
      }
      activeIntervals.clear();
      activeTimeouts.clear();
      restoreGlobals(previous);
    },
  };
}

export async function loadUserShellApp(options = {}) {
  return loadShellApp("user", options);
}

export async function loadHubShellApp(options = {}) {
  return loadShellApp("hub", options);
}

export async function loadAdminShellApp(options = {}) {
  return loadShellApp("admin", options);
}

export async function loadUnifiedShellApp(options = {}) {
  return loadShellApp("unified", options);
}

function createLocalStorage() {
  const map = new Map();
  return {
    getItem(key) {
      return map.has(key) ? map.get(key) : null;
    },
    setItem(key, value) {
      map.set(String(key), String(value));
    },
    removeItem(key) {
      map.delete(key);
    },
    clear() {
      map.clear();
    },
  };
}

function captureGlobals() {
  return {
    window: globalThis.window,
    document: globalThis.document,
    HTMLElement: globalThis.HTMLElement,
    Element: globalThis.Element,
    Event: globalThis.Event,
    CustomEvent: globalThis.CustomEvent,
    navigator: globalThis.navigator,
    fetch: globalThis.fetch,
    localStorage: globalThis.localStorage,
    setTimeout: globalThis.setTimeout,
    clearTimeout: globalThis.clearTimeout,
    setInterval: globalThis.setInterval,
    clearInterval: globalThis.clearInterval,
    requestAnimationFrame: globalThis.requestAnimationFrame,
    cancelAnimationFrame: globalThis.cancelAnimationFrame,
  };
}

function restoreGlobals(snapshot) {
  for (const [key, value] of Object.entries(snapshot)) {
    if (value === undefined) {
      delete globalThis[key];
    } else {
      globalThis[key] = value;
    }
  }
}

async function waitFor(predicate, timeoutMs = 2000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error("timed out waiting for app initialization");
}
