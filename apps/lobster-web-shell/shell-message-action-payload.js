/* ============================================================
   shell-message-action-payload.js — 消息发送/编辑/撤回 payload 纯构造
   从 app.js 提取。返回纯 payload 对象，无 DOM / 无 fetch / 无状态变更。
   currentIdentity / languageTag 通过 deps 注入，脱离全局即可单测。
   ============================================================ */

export function gatewayMessagePayloadForState(roomId, text, quickAction, deps, attachmentId = "") {
  const attachment = typeof attachmentId === "string" ? attachmentId.trim() : "";
  return {
    room_id: roomId,
    sender: deps.currentIdentity(),
    text,
    attachment_id: attachment || undefined,
    quick_action: quickAction || undefined,
    device_id: "browser-shell",
    language_tag: deps.languageTag || "zh-CN",
  };
}

export function editMessagePayloadForState(roomId, messageId, text, deps) {
  return {
    room_id: roomId,
    message_id: messageId,
    actor: deps.currentIdentity(),
    text: text.trim(),
  };
}

export function recallMessagePayloadForState(roomId, messageId, deps) {
  return {
    room_id: roomId,
    message_id: messageId,
    actor: deps.currentIdentity(),
  };
}
