/* sw.js — WebPush service worker（最小实现）。
 * 只做两件事：push 事件展示系统通知；点击通知聚焦/打开主城页。
 * 不做离线缓存——缓存纪律由 CF/nginx 的 no-cache + ?v= 升版承担。
 */

self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

function pushTargetUrl() {
  return "./index.html";
}

self.addEventListener("push", (event) => {
  let title = "我和狗蛋儿的家";
  let body = "有新消息";
  let tag = "lobster-chat";
  try {
    if (event.data) {
      const payload = event.data.json();
      if (typeof payload.title === "string" && payload.title) title = payload.title;
      if (typeof payload.body === "string" && payload.body) body = payload.body;
      if (typeof payload.tag === "string" && payload.tag) tag = payload.tag;
    }
  } catch (error) {
    // 载荷解析失败仍展示兜底文案
  }
  event.waitUntil(
    self.registration.showNotification(title, {
      body,
      tag,
      renotify: true,
      icon: "./assets/icons/icon-192.png",
      badge: "./assets/icons/icon-192.png",
      data: { url: pushTargetUrl() },
    }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const target = (event.notification.data && event.notification.data.url) || pushTargetUrl();
  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clientList) => {
      for (const client of clientList) {
        if (!client.url.includes(target)) continue;
        return client.focus();
      }
      return self.clients.openWindow(target);
    }),
  );
});
