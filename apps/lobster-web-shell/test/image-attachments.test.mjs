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
