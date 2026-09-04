// 图片消息点击看原图的灯箱遮罩(气泡里是 object-fit: cover 裁切缩略，
// 灯箱里 object-fit: contain 展示完整原图)。
// 纯 DOM 模块,不依赖 app.js 全局,便于 fake-dom 单测。
// 视觉规范:dark-on-dark,禁大块金色/cream。

export function createAttachmentLightbox({ document: doc = document } = {}) {
  const mask = doc.createElement("div");
  mask.className = "attachment-lightbox-mask";
  mask.hidden = true;

  const image = doc.createElement("img");
  image.className = "attachment-lightbox-image";
  image.alt = "图片原文";

  mask.appendChild(image);

  function close() {
    mask.hidden = true;
    image.removeAttribute("src");
  }

  mask.addEventListener("click", close);
  if (doc && typeof doc.addEventListener === "function") {
    doc.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && !mask.hidden) close();
    });
  }

  function open(src) {
    const raw = typeof src === "string" ? src.trim() : "";
    if (!raw) return false;
    image.setAttribute("src", raw);
    mask.hidden = false;
    return true;
  }

  return { element: mask, open, close, isOpen: () => !mask.hidden };
}

// 全局事件委托:点击气泡内的附件图片打开灯箱。返回 wire 函数便于单测。
export function wireAttachmentLightbox(lightbox, { document: doc = document } = {}) {
  doc.addEventListener("click", (event) => {
    const target = event.target;
    const image = target && typeof target.closest === "function"
      ? target.closest("img.message-attachment")
      : null;
    if (!image) return;
    const src = typeof image.getAttribute === "function" ? image.getAttribute("src") : "";
    if (lightbox.open(src)) event.preventDefault();
  });
  return lightbox;
}
