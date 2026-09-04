/* shell-image-compress.js — 发送前图片压缩（纯函数决策 + 浏览器执行）
 * 决策函数 imageCompressionPlan 无 DOM / 无副作用，可直接 node 单测；
 * compressImageFile 负责浏览器位图执行，任何一步失败都回退原图
 * （fail-open，Gateway 仍有魔数嗅探 + 5MB 上限兜底）。
 * 合同：gif 永不重编码（保动画）；png 只降尺寸不换格式（保透明通道）；
 * jpeg/webp 大图重编码为 jpeg（手机照片体积收益最大）。
 */

export const IMAGE_COMPRESS_MAX_DIMENSION = 2048;
export const IMAGE_COMPRESS_PASSTHROUGH_MAX_BYTES = 512 * 1024;
export const IMAGE_COMPRESS_JPEG_QUALITY = 0.85;

export function imageCompressionPlan({ mime, byteSize, width = 0, height = 0 } = {}) {
  const normalized = typeof mime === "string" ? mime.toLowerCase() : "";
  if (normalized === "image/gif") {
    return { mode: "passthrough", reason: "gif-animation" };
  }
  const maxDimension = Math.max(width || 0, height || 0);
  const withinBudget =
    (byteSize || 0) <= IMAGE_COMPRESS_PASSTHROUGH_MAX_BYTES &&
    (maxDimension === 0 || maxDimension <= IMAGE_COMPRESS_MAX_DIMENSION);
  if (normalized === "image/png") {
    if (withinBudget) return { mode: "passthrough", reason: "png-within-budget" };
    if (maxDimension > IMAGE_COMPRESS_MAX_DIMENSION) {
      return {
        mode: "reencode",
        mime: "image/png",
        maxDimension: IMAGE_COMPRESS_MAX_DIMENSION,
        reason: "png-oversize-dimensions",
      };
    }
    return { mode: "passthrough", reason: "png-lossless-keep" };
  }
  if (normalized === "image/jpeg" || normalized === "image/webp") {
    if (withinBudget) return { mode: "passthrough", reason: "photo-within-budget" };
    return {
      mode: "reencode",
      mime: "image/jpeg",
      quality: IMAGE_COMPRESS_JPEG_QUALITY,
      maxDimension: IMAGE_COMPRESS_MAX_DIMENSION,
      reason: "photo-reencode",
    };
  }
  return { mode: "passthrough", reason: "mime-not-recognized" };
}

function decodeImageBitmap(file, bitmapFactory) {
  if (typeof bitmapFactory !== "function") return Promise.resolve(null);
  return bitmapFactory(file).catch(() => null);
}

function encodeCanvasBlob(canvas, mime, quality, doc) {
  if (typeof canvas.convertToBlob === "function") {
    return canvas.convertToBlob({ type: mime, quality }).then((blob) => blob || null);
  }
  if (typeof doc !== "undefined" && typeof canvas.toBlob === "function") {
    return new Promise((resolve) => {
      canvas.toBlob((blob) => resolve(blob || null), mime, quality);
    });
  }
  return Promise.resolve(null);
}

export async function compressImageFile(file, deps = {}) {
  if (!file || typeof file.size !== "number") return file;
  const bitmapFactory = deps.createImageBitmap ?? globalThis.createImageBitmap;
  const doc = deps.document ?? (typeof document !== "undefined" ? document : undefined);
  const bitmap = await decodeImageBitmap(file, bitmapFactory).catch(() => null);
  if (!bitmap || !bitmap.width || !bitmap.height) return file;
  const plan = imageCompressionPlan({
    mime: file.type,
    byteSize: file.size,
    width: bitmap.width,
    height: bitmap.height,
  });
  if (plan.mode !== "reencode") return file;
  try {
    const scale = Math.min(
      1,
      plan.maxDimension / Math.max(bitmap.width, bitmap.height),
    );
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));
    let canvas;
    let context;
    if (typeof OffscreenCanvas === "function") {
      canvas = new OffscreenCanvas(width, height);
      context = canvas.getContext("2d");
    } else if (doc && typeof doc.createElement === "function") {
      canvas = doc.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      context = canvas.getContext("2d");
    }
    if (!canvas || !context) return file;
    context.drawImage(bitmap, 0, 0, width, height);
    if (typeof bitmap.close === "function") bitmap.close();
    const blob = await encodeCanvasBlob(canvas, plan.mime, plan.quality, doc);
    if (!blob || blob.size <= 0 || blob.size >= file.size) return file;
    const name = typeof file.name === "string" && file.name ? file.name : "image";
    return new File([blob], name, { type: plan.mime });
  } catch {
    return file;
  }
}
