/* shell-message-body.js — 消息体 DOM spec 构造纯函数
 * 从 app.js 提取。返回纯数据 spec 树 { tag, className, dataset, text, children }，
 * app.js 负责递归落地为真实 DOM 节点。无 DOM / 无副作用。
 * 依赖 quickActionIntensity/Tone/FollowUpLabel/Copy + parseStructuredQuickActionMessage
 * 均从既有模块 import。
 */

import {
  quickActionIntensity,
  quickActionTone,
  quickActionFollowUpLabel,
  quickActionFollowUpCopy,
} from "./shell-quick-action-labels.js";
import { parseStructuredQuickActionMessage } from "./shell-quick-action-preview.js";

function resolveAttachmentSrc(url, attachmentBase) {
  const raw = typeof url === "string" ? url.trim() : "";
  if (!raw) return "";
  if (/^https?:\/\//i.test(raw) || raw.startsWith("data:") || raw.startsWith("blob:")) {
    return raw;
  }
  const base = typeof attachmentBase === "string" ? attachmentBase.replace(/\/+$/, "") : "";
  return base ? `${base}${raw.startsWith("/") ? raw : `/${raw}`}` : raw;
}

export function messageAttachmentDomSpec(message, options = {}) {
  const attachment = message?.attachment;
  const src = resolveAttachmentSrc(attachment?.url, options.attachmentBase);
  if (!src) return null;
  return {
    tag: "img",
    className: "message-attachment",
    attrs: {
      src,
      alt: "图片消息",
      loading: "lazy",
      "data-attachment-mime": typeof attachment.mime_type === "string" ? attachment.mime_type : "",
    },
  };
}

function quickActionChipSpec({ className, action, label }) {
  return {
    tag: "span",
    className,
    text: label,
    dataset: {
      actionIntensity: quickActionIntensity(action),
      quickAction: action,
    },
    extraClass: `message-quick-action-${quickActionTone(action)}`,
  };
}

// 构造 body 外壳 spec（含 quick_action dataset）
function messageBodyShellSpec(structured, action) {
  const dataset = action
    ? { quickAction: action, actionIntensity: quickActionIntensity(action) }
    : undefined;
  return {
    tag: "div",
    className: structured ? "message-body message-body-structured" : "message-body",
    dataset,
  };
}

// 结构化 sheet 的 fields 行 spec
function quickSheetFieldRowSpecs(structured) {
  return structured.fields.map((field) => ({
    tag: "div",
    className: "message-quick-sheet-row",
    children: [
      { tag: "span", className: "message-quick-sheet-label", text: field.label },
      { tag: "span", className: "message-quick-sheet-value", text: field.value },
    ],
  }));
}

function quickSheetNotesSpec(structured) {
  if (!structured.notes.length) return null;
  return {
    tag: "div",
    className: "message-quick-sheet-notes",
    text: structured.notes.join("\n"),
  };
}

function quickSheetFollowUpSpec(action, quickState) {
  const followUpLabel = quickActionFollowUpLabel(action, quickState);
  const followUpCopy = quickActionFollowUpCopy(action, quickState);
  if (!followUpLabel || !followUpCopy) return null;
  return {
    tag: "div",
    className: "message-quick-sheet-follow-up",
    children: [
      { tag: "span", className: "message-quick-sheet-follow-up-label", text: followUpLabel },
      { tag: "span", className: "message-quick-sheet-follow-up-copy", text: followUpCopy },
    ],
  };
}

function structuredSheetSpec(structured, action, quickState) {
  const children = [
    ...quickSheetFieldRowSpecs(structured),
    quickSheetNotesSpec(structured),
    quickSheetFollowUpSpec(action, quickState),
  ].filter(Boolean);
  return { tag: "div", className: "message-quick-sheet", children };
}

export function messageBodyDomSpec(message, options = {}) {
  const structured = parseStructuredQuickActionMessage(message);
  const action = typeof message?.quick_action === "string" ? message.quick_action.trim() : "";
  const quickState = typeof options.quickState === "string" ? options.quickState : "";

  // 终态优先
  if (message?.is_recalled) {
    return { tag: "div", className: "message-body message-body-recalled", text: "消息已撤回" };
  }
  if (message?.moderation_status === "blocked") {
    return { tag: "div", className: "message-body message-body-recalled", text: "消息已屏蔽" };
  }

  const shell = messageBodyShellSpec(structured, action);
  if (!structured) {
    const attachmentSpec = messageAttachmentDomSpec(message, options);
    if (attachmentSpec) {
      shell.children = [attachmentSpec];
      if (typeof message?.text === "string" && message.text.trim()) {
        shell.children.push({ tag: "div", className: "message-attachment-caption", text: message.text });
      }
      return shell;
    }
    shell.text = message?.text;
    return shell;
  }
  shell.children = [structuredSheetSpec(structured, action, quickState)];
  return shell;
}

// 暴露 chip spec 构造器供 app.js 落地 quick-action/state chip（保留原 createMessageQuickActionChip/StateChip 语义）
export function messageQuickActionChipSpec(action) {
  if (!action) return null;
  return quickActionChipSpec({ className: "message-quick-action", action, label: action });
}

export function messageQuickStateChipSpec(action, state = "") {
  const label = quickActionFollowUpLabel(action, state);
  if (!label) return null;
  return quickActionChipSpec({ className: "message-quick-state", action, label });
}
