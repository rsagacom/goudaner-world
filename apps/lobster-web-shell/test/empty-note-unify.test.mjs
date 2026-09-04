/*
   empty-note-unify.test.mjs — 2026-09-04 empty-note 视觉统一静态合同

   背景：`.empty-note`（房间/居民/世界/治理列表空态）此前只在 styles.chat.css
   定义，而渲染它的 app.js 同时被 index/admin/creative/unified 四个页面加载，
   其中 creative/unified 不加载 chat.css，空态实际无样式。统一后唯一真源移到
   所有 app.js 页面共同加载的 styles.creative.css，并以 `:not(.timeline-empty-card)`
   排除时间线空态卡片，保留其自身卡片合同。
*/
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

async function readShellModule(name) {
  return fs.readFile(new URL(`../${name}`, import.meta.url), "utf8");
}

test("creative.css is the single canonical home for .empty-note list states", async () => {
  const css = await readShellModule("styles.creative.css");
  assert.match(css, /\.empty-note:not\(\.timeline-empty-card\) \{/);
  assert.match(css, /\.empty-note:not\(\.timeline-empty-card\) \{[\s\S]*?text-align: center/);
  assert.match(css, /\.empty-note:not\(\.timeline-empty-card\) \{[\s\S]*?background: rgba\(22, 16, 12, 0\.55\)/);
  assert.match(css, /\.empty-note:not\(\.timeline-empty-card\) \{[\s\S]*?border: 1px solid #3a2f28/);
});

test("chat.css no longer defines a bare .empty-note rule", async () => {
  const css = await readShellModule("styles.chat.css");
  assert.doesNotMatch(css, /^\.empty-note \{/m);
});

test("every app.js page references the unified creative.css cache version", async () => {
  for (const page of ["index.html", "admin.html", "creative.html", "admin-ds.html", "unified.html"]) {
    const html = await readShellModule(page);
    assert.match(html, /styles\.creative\.css\?v=20260905-attach-fallback/, `${page} must bump creative.css cache version`);
  }
});

test("timeline empty cards keep their own class contract untouched", async () => {
  const spec = await readShellModule("shell-timeline-empty-state.js");
  assert.match(spec, /className: "empty-note timeline-empty timeline-empty-card"/);
  const locked = await readShellModule("shell-private-room-locked-card.js");
  assert.match(locked, /timeline-empty-card private-room-locked-card/);
});
