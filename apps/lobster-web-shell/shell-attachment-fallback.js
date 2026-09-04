// shell-attachment-fallback.js — 附件图片加载失败时的占位降级。
// 网关不可达/附件被清理/网络错误时，<img> 会显示浏览器破图标；
// 这里在捕获阶段接管 error 事件，把破图替换为深色占位节点。
// 纯 DOM 模块,不依赖 app.js 全局,便于 fake-dom 单测。

export function createAttachmentErrorFallback({ document: doc = document } = {}) {
  function makeFallbackNode() {
    const node = doc.createElement("div");
    node.className = "message-attachment-failed";
    node.textContent = "图片无法加载";
    return node;
  }

  function isAttachmentImage(target) {
    return Boolean(
      target &&
        typeof target.classList?.contains === "function" &&
        target.classList.contains("message-attachment") &&
        typeof target.replaceWith === "function",
    );
  }

  function handleError(event) {
    const image = event?.target;
    if (!isAttachmentImage(image)) return;
    image.replaceWith(makeFallbackNode());
  }

  // 资源 error 事件不冒泡，必须捕获阶段接管。
  doc.addEventListener("error", handleError, true);

  return { handleError };
}

// 页面接线。
export function installAttachmentErrorFallback(context = {}) {
  const doc = context.document ?? document;
  return createAttachmentErrorFallback({ document: doc });
}
