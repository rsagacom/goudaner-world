/* ============================================================
   image-attachments.test.mjs — 图片消息投影与发送合同测试
   覆盖：shell-message-body 的 attachment DOM spec（img + caption、
        base 拼接、无附件回退、撤回丢弃）、
        shell-message-action-payload 的 attachment_id 透传（空值省略）、
        shell-message-send 控制器对 attachmentId 的传递。
   ============================================================ */

import test from "node:test";
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";

const bodyUrl = new URL("../shell-message-body.js", import.meta.url);
const bodyMod = await import(pathToFileURL(bodyUrl.pathname).href);
const { messageBodyDomSpec } = bodyMod;

const payloadUrl = new URL("../shell-message-action-payload.js", import.meta.url);
const payloadMod = await import(pathToFileURL(payloadUrl.pathname).href);
const { gatewayMessagePayloadForState } = payloadMod;

const sendUrl = new URL("../shell-message-send.js", import.meta.url);
const { createMessageSendController } = await import(pathToFileURL(sendUrl.pathname).href);

// ---- 消息体附件渲染 ----

test("附件消息渲染 img spec 并解析相对 URL", () => {
  const spec = messageBodyDomSpec(
    { text: "看这张", attachment: { url: "/v1/shell/attachment/abc", mime_type: "image/png" } },
    { attachmentBase: "https://chat.example.com" },
  );
  const img = spec.children?.[0];
  assert.equal(img.tag, "img");
  assert.equal(img.className, "message-attachment");
  assert.equal(img.attrs.src, "https://chat.example.com/v1/shell/attachment/abc");
  assert.equal(img.attrs.loading, "lazy");
  assert.equal(spec.children[1].className, "message-attachment-caption");
  assert.equal(spec.children[1].text, "看这张");
});

test("无 caption 的附件消息只渲染图片", () => {
  const spec = messageBodyDomSpec(
    { text: "", attachment: { url: "/v1/shell/attachment/abc", mime_type: "image/png" } },
    { attachmentBase: "https://chat.example.com" },
  );
  assert.equal(spec.children.length, 1);
  assert.equal(spec.children[0].tag, "img");
});

test("同源部署不加 attachmentBase 时保留相对路径", () => {
  const spec = messageBodyDomSpec(
    { text: "", attachment: { url: "/v1/shell/attachment/abc", mime_type: "image/png" } },
  );
  assert.equal(spec.children[0].attrs.src, "/v1/shell/attachment/abc");
});

test("无附件的普通文本不受影响", () => {
  const spec = messageBodyDomSpec({ text: "你好" });
  assert.equal(spec.text, "你好");
  assert.equal(spec.children, undefined);
});

test("撤回消息仍显示撤回文案而不渲染图片", () => {
  const spec = messageBodyDomSpec(
    { is_recalled: true, text: "", attachment: { url: "/v1/shell/attachment/abc" } },
  );
  assert.equal(spec.text, "消息已撤回");
  assert.equal(spec.children, undefined);
});

// ---- payload 透传 ----

test("payload 带 attachment_id 且空值省略", () => {
  const deps = { currentIdentity: () => "rsaga", languageTag: "zh-CN" };
  const withAttachment = gatewayMessagePayloadForState("room:world:lobby", "x", "", deps, "abc123");
  assert.equal(withAttachment.attachment_id, "abc123");
  const without = gatewayMessagePayloadForState("room:world:lobby", "x", "", deps);
  assert.equal(without.attachment_id, undefined);
});

// ---- 发送控制器透传 ----

test("send 控制器把 attachmentId 传进 payload", async () => {
  let captured = null;
  const controller = createMessageSendController({
    getContext: () => ({ roomId: "room:world:lobby", gatewayConnected: true }),
    buildPayload: (request) => ({ attachment_id: request.attachmentId || undefined }),
    prepareGateway: () => "pending-1",
    postGateway: async ({ payload }) => {
      captured = payload;
    },
    refreshGateway: async () => {},
    clearPending: () => {},
  });
  await controller.send("", { attachmentId: "abc123" });
  assert.deepEqual(captured, { attachment_id: "abc123" });
});

// ---- 发送前压缩决策（shell-image-compress 纯函数） ----

const compressUrl = new URL("../shell-image-compress.js", import.meta.url);
const { imageCompressionPlan, IMAGE_COMPRESS_MAX_DIMENSION } = await import(
  pathToFileURL(compressUrl.pathname).href
);

test("gif 永远直传以保留动画", () => {
  const plan = imageCompressionPlan({ mime: "image/gif", byteSize: 8 * 1024 * 1024, width: 4000, height: 3000 });
  assert.equal(plan.mode, "passthrough");
  assert.equal(plan.reason, "gif-animation");
});

test("预算内图片直传", () => {
  const small = imageCompressionPlan({ mime: "image/jpeg", byteSize: 300 * 1024, width: 1600, height: 900 });
  assert.equal(small.mode, "passthrough");
  const pngSmall = imageCompressionPlan({ mime: "image/png", byteSize: 300 * 1024, width: 1600, height: 900 });
  assert.equal(pngSmall.mode, "passthrough");
});

test("大 jpeg/webp 重编码为限尺寸 jpeg", () => {
  const plan = imageCompressionPlan({ mime: "image/jpeg", byteSize: 4 * 1024 * 1024, width: 4032, height: 3024 });
  assert.equal(plan.mode, "reencode");
  assert.equal(plan.mime, "image/jpeg");
  assert.equal(plan.maxDimension, IMAGE_COMPRESS_MAX_DIMENSION);
  const webp = imageCompressionPlan({ mime: "image/webp", byteSize: 2 * 1024 * 1024, width: 3000, height: 2000 });
  assert.equal(webp.mode, "reencode");
  assert.equal(webp.mime, "image/jpeg");
});

test("png 超尺寸只降尺寸保持 png（保透明通道），大而小尺寸的 png 直传", () => {
  const oversized = imageCompressionPlan({ mime: "image/png", byteSize: 3 * 1024 * 1024, width: 5000, height: 2800 });
  assert.equal(oversized.mode, "reencode");
  assert.equal(oversized.mime, "image/png");
  const lossless = imageCompressionPlan({ mime: "image/png", byteSize: 2 * 1024 * 1024, width: 1800, height: 1000 });
  assert.equal(lossless.mode, "passthrough");
  assert.equal(lossless.reason, "png-lossless-keep");
});

test("未知 mime 回退直传", () => {
  const plan = imageCompressionPlan({ mime: "image/avif", byteSize: 3 * 1024 * 1024, width: 4000, height: 3000 });
  assert.equal(plan.mode, "passthrough");
  assert.equal(plan.reason, "mime-not-recognized");
});

// ---- 点击看原图灯箱（shell-attachment-lightbox） ----

const lightboxUrl = new URL("../shell-attachment-lightbox.js", import.meta.url);
const { createAttachmentLightbox, wireAttachmentLightbox } = await import(
  pathToFileURL(lightboxUrl.pathname).href
);

function lightboxHarness() {
  const docListeners = new Map();
  const doc = {
    createElement: (tag) => ({
      tagName: tag.toUpperCase(),
      className: "",
      hidden: false,
      alt: "",
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
      removeAttribute(name) {
        delete this.attrs[name];
      },
      addEventListener(type, handler) {
        this.listeners.set(type, handler);
      },
      click() {
        this.listeners.get("click")?.({ target: this });
      },
    }),
    addEventListener(type, handler) {
      docListeners.set(type, handler);
    },
  };
  const lightbox = createAttachmentLightbox({ document: doc });
  wireAttachmentLightbox(lightbox, { document: doc });
  return { lightbox, doc, docListeners };
}

test("灯箱打开与关闭（遮罩点击 / Esc / 空 src 拒绝）", () => {
  const { lightbox, docListeners } = lightboxHarness();
  assert.equal(lightbox.open(""), false);
  assert.equal(lightbox.open("https://chat.example.com/v1/shell/attachment/abc"), true);
  assert.equal(lightbox.isOpen(), true);
  assert.equal(
    lightbox.element.children[0].attrs.src,
    "https://chat.example.com/v1/shell/attachment/abc",
  );

  docListeners.get("keydown")({ key: "Escape" });
  assert.equal(lightbox.isOpen(), false);
  assert.equal(lightbox.element.children[0].attrs.src, undefined);

  lightbox.open("/v1/shell/attachment/def");
  lightbox.element.click();
  assert.equal(lightbox.isOpen(), false);
});

test("点击气泡内附件图片打开灯箱并阻止默认跳转", () => {
  const { lightbox, docListeners } = lightboxHarness();
  const src = "/v1/shell/attachment/abc";
  const image = {
    className: "message-attachment",
    attrs: { src },
    closest(selector) {
      return selector === "img.message-attachment" ? this : null;
    },
    getAttribute(name) {
      return this.attrs[name] ?? null;
    },
  };
  let defaultPrevented = false;
  docListeners.get("click")({
    target: image,
    preventDefault: () => {
      defaultPrevented = true;
    },
  });
  assert.equal(defaultPrevented, true);
  assert.equal(lightbox.isOpen(), true);
  assert.equal(lightbox.element.children[0].attrs.src, src);

  const outside = {
    className: "message-body",
    closest() {
      return null;
    },
  };
  docListeners.get("click")({ target: outside, preventDefault: () => {} });
  assert.equal(lightbox.isOpen(), true);
});
