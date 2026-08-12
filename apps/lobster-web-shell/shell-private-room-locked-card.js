/**
 * 未授权私宅 stage 空态卡片纯模型。
 *
 * 背景：点击未授权私宅此前只有 governance 状态条一行字，stage 区域没有任何反馈。
 * 这里把 residentPrivateRoomAccessPromptModel 的五态文案升级为 timeline 区居中卡片，
 * 复用 timeline-empty-card 结构（title/copy/action），只产出纯数据，DOM 由 app.js 挂载。
 */

const TONE_TITLE = {
  "is-locked": "这道门暂时进不去",
  "is-pending": "还差一步",
  "is-actionable": "对方在等你回应",
};

function actionTextFor(prompt, displayName) {
  if (!prompt) return "";
  if (prompt.isError && prompt.className.includes("is-locked")) {
    // 未登录 / 需要好友两种 locked 都由 prompt.text 说明原因，这里给下一步
    return prompt.text.startsWith("登录后")
      ? "先登录居民身份，再按房主策略申请进入。"
      : `点「申请好友」，等 ${displayName} 接受后再来。`;
  }
  if (prompt.className.includes("is-actionable")) {
    return "在居民列表点「接受好友」，通过后即可进入。";
  }
  if (prompt.className.includes("is-pending")) {
    return "无需重复申请，对方接受后私宅入口会自动出现。";
  }
  return "";
}

export function privateRoomLockedCardModel(
  prompt,
  { displayName = "" } = {},
) {
  if (!prompt) return null;
  const tone =
    ["is-locked", "is-pending", "is-actionable"].find((t) =>
      prompt.className.includes(t),
    ) || "is-locked";
  return {
    className: `empty-note timeline-empty timeline-empty-card private-room-locked-card ${tone}`,
    titleClassName: "timeline-empty-title",
    titleText: TONE_TITLE[tone],
    copyClassName: "timeline-empty-copy",
    copyText: prompt.text,
    actionClassName: "timeline-empty-action",
    actionText: actionTextFor(prompt, displayName || "对方"),
  };
}
