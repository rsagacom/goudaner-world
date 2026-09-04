import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const rootDir = path.dirname(fileURLToPath(import.meta.url));

const MIME_TYPES = new Map([
  [".html", "text/html; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".mjs", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".png", "image/png"],
  [".avif", "image/avif"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".svg", "image/svg+xml"],
]);

function createStaticServer() {
  return createServer(async (req, res) => {
    try {
      const url = new URL(req.url || "/", "http://127.0.0.1");
      const pathname = url.pathname === "/" ? "/index.html" : decodeURIComponent(url.pathname);
      const candidate = path.resolve(rootDir, `.${pathname}`);
      if (!candidate.startsWith(rootDir)) {
        res.writeHead(403).end("Forbidden");
        return;
      }
      const body = await readFile(candidate);
      res.writeHead(200, {
        "content-type": MIME_TYPES.get(path.extname(candidate)) || "application/octet-stream",
      });
      res.end(body);
    } catch {
      res.writeHead(404).end("Not found");
    }
  });
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      resolve(server.address());
    });
  });
}

function close(server) {
  return new Promise((resolve) => server.close(resolve));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

async function waitForApp(page) {
  await page.waitForSelector("#composer-input:not([disabled])", { timeout: 5000 });
  await page.waitForSelector('.scene-hotspot-label-chip[data-hotspot-title="吧台"]', { state: "attached", timeout: 5000 });
}

async function visibleOpacity(page, selector) {
  return Number(await page.locator(selector).evaluate((node) => getComputedStyle(node).opacity));
}

async function elementBox(page, selector) {
  const box = await page.locator(selector).boundingBox();
  assert(box, `missing element box: ${selector}`);
  return box;
}

async function verifyCreativeComposer(page, baseUrl) {
  await page.setViewportSize({ width: 1259, height: 872 });
  await page.goto(`${baseUrl}/creative.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await waitForApp(page);

  const input = page.locator("#composer-input");
  await input.fill("真实输入测试");
  assert(await input.inputValue() === "真实输入测试", "creative composer should show typed text");

  const textColor = await input.evaluate((node) => getComputedStyle(node).color);
  assert(!/rgba?\(0,\s*0,\s*0,\s*0\)/.test(textColor), "creative composer text must not be transparent");

  await input.press("Shift+Enter");
  assert((await input.inputValue()).includes("\n"), "Shift+Enter should keep a multiline draft");

  await input.fill("回车发送真实验收");
  await input.press("Enter");
  await page.waitForFunction(
    () => Array.from(document.querySelectorAll("#timeline .message-body")).some((node) =>
      node.textContent?.includes("回车发送真实验收"),
    ),
    null,
    { timeout: 5000 },
  );
  assert(await input.inputValue() === "", "Enter send should clear the composer");
}

async function verifyCreativeHotspots(page, baseUrl) {
  await page.goto(`${baseUrl}/creative.html?verify=frontend-realness-hotspots`, { waitUntil: "networkidle" });
  await page.evaluate(() => {
    try {
      localStorage.clear();
    } catch {
      // Ignore storage restrictions in browser verification.
    }
  });
  await page.reload({ waitUntil: "networkidle" });
  await waitForApp(page);

  const coffeeLabel = '.scene-hotspot-label-chip[data-hotspot-title="吧台"]';
  assert(await visibleOpacity(page, coffeeLabel) === 0, "hotspot labels should not permanently cover the scene");

  await page.locator(".scene-hotspot--coffee").hover();
  await page.waitForFunction(
    (selector) => Number(getComputedStyle(document.querySelector(selector)).opacity) > 0.9,
    coffeeLabel,
    { timeout: 1000 },
  );
  assert(await visibleOpacity(page, coffeeLabel) > 0.9, "hotspot hover should reveal its label");

  await page.mouse.move(12, 12);
  await page.waitForTimeout(120);
  assert(await visibleOpacity(page, coffeeLabel) === 0, "hotspot label should hide after pointer leaves");

  const stage = await elementBox(page, ".creative-stage");
  await page.mouse.click(stage.x + stage.width * 0.58, stage.y + stage.height * 0.62);
  await page.waitForFunction(() => document.body.classList.contains("scene-hotspot-labels-visible"));
  await page.waitForFunction(() => document.body.classList.contains("scene-clear-mode"));
  await page.waitForFunction(
    (selector) => Number(getComputedStyle(document.querySelector(selector)).opacity) > 0.9,
    coffeeLabel,
    { timeout: 1000 },
  );
  assert(await visibleOpacity(page, coffeeLabel) > 0.9, "blank scene click should reveal all hotspot labels");
  await page.waitForFunction(
    (selector) => Number(getComputedStyle(document.querySelector(selector)).opacity) < 0.05,
    ".creative-chat-frame",
    { timeout: 1000 },
  );
  assert(await visibleOpacity(page, ".creative-chat-frame") < 0.05, "blank scene click should hide chat chrome");
}

async function verifySceneRails(page, baseUrl) {
  const cases = [
    { path: "/creative.html", rail: "#creative-rail", stage: ".creative-stage" },
    { path: "/index.html", rail: ".public-square-rail", stage: ".public-square-stage" },
    { path: "/world-square.html", rail: ".world-square-rail", stage: ".world-square-stage" },
  ];
  for (const item of cases) {
    await page.setViewportSize({ width: 1560, height: 873 });
    await page.goto(`${baseUrl}${item.path}?verify=frontend-realness`, { waitUntil: "networkidle" });
    const rail = await elementBox(page, item.rail);
    const stage = await elementBox(page, item.stage);
    assert(Math.abs(rail.width - 220) <= 1, `${item.path} rail width should stay on the shared 220px token`);
    assert(Math.abs(rail.height - stage.height) <= 1, `${item.path} rail and stage should align vertically`);
  }
}

async function verifyDayNightBackgrounds(page, baseUrl) {
  const cases = [
    { path: "/creative.html", selector: ".creative-stage", expected: "creative-room-scene-v2-day" },
    { path: "/index.html", selector: ".public-square-stage", expected: "hub-main-city-scene-v1-day" },
    { path: "/world-square.html", selector: ".world-square-scene", expected: "world-square-concept-day", pseudo: "::before" },
  ];

  for (const c of cases) {
    await page.setViewportSize({ width: 1560, height: 873 });
    await page.goto(`${baseUrl}${c.path}?verify=frontend-realness`, { waitUntil: "networkidle" });

    const timeOfDay = await page.evaluate(() => document.body?.dataset?.timeOfDay);
    assert(timeOfDay === "day" || timeOfDay === "night", `${c.path} body must have data-time-of-day`);

    let bg;
    if (c.pseudo) {
      bg = await page.evaluate((sel) => {
        const el = document.querySelector(sel);
        if (!el) return "not found";
        return getComputedStyle(el, "::before").backgroundImage;
      }, c.selector);
    } else {
      bg = await page.locator(c.selector).evaluate((node) => getComputedStyle(node).backgroundImage);
    }

    const expectedAsset = c.expected.replace("-day", timeOfDay === "day" ? "-day" : "");
    assert(bg.includes(expectedAsset), `${c.path} must load ${timeOfDay} background asset (${expectedAsset}), got: ${bg.slice(0, 120)}`);
  }
}

async function verifyAdminDs(page, baseUrl) {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${baseUrl}/admin-ds.html?verify=frontend-realness`, { waitUntil: "networkidle" });

  assert(await page.title() === "AJW聊天 · 管理后台", "admin-ds should be the formal admin page");
  await page.locator('[data-module="residents"]').click();
  await page.waitForFunction(() => document.querySelector("#mod-residents")?.classList.contains("active"));

  const search = page.locator("#residentSearch");
  await search.fill("chen");
  await page.waitForFunction(() => {
    const rows = Array.from(document.querySelectorAll("#residentTableBody tr"));
    return rows.length > 0 && rows.every((row) => /chen/i.test(row.textContent || ""));
  });
  assert(!(await page.locator('[data-admin-action="create-resident"]').isDisabled()), "admin-ds create-resident should be enabled now that Gateway write API exists");
  assert(/前端分页/.test(await page.locator("#mod-residents .ds-pagination-info").textContent()), "admin-ds pagination should be labeled as client-side pagination");
  var pageBtnCount = await page.locator("#mod-residents .ds-page-btn:not([disabled])").count();
  assert(pageBtnCount > 0, "admin-ds pagination should have at least one clickable page button");

  await page.locator("#residentTableBody tr").first().click();
  await page.waitForFunction(() => !document.querySelector("#dsDetailPanel")?.classList.contains("hidden"));
  assert(
    /居民:/.test(await page.locator("#dsDetailTitle").textContent()),
    "admin-ds detail panel should open from a resident row",
  );
  await page.locator("#dsDetailActions button", { hasText: "查看会话" }).click();
  await page.waitForFunction(() => document.querySelector("#mod-rooms")?.classList.contains("active"));
  await page.waitForFunction(() => document.querySelector("#dsDetailPanel")?.classList.contains("hidden"));

  await page.locator('[data-module="rooms"]').click();
  await page.waitForFunction(() => document.querySelector("#mod-rooms")?.classList.contains("active"));
  await page.locator("#roomTypeFilter").selectOption("private");
  await page.waitForFunction(() => {
    const rows = Array.from(document.querySelectorAll("#roomTableBody tr"));
    return rows.length > 0 && rows.every((row) => /私聊/.test(row.textContent || ""));
  });

  await page.setViewportSize({ width: 390, height: 820 });
  await page.reload({ waitUntil: "networkidle" });
  await page.waitForFunction(() => document.querySelector("#dsSidebar")?.classList.contains("collapsed"));
  const mobileLayout = await page.evaluate(() => {
    const sidebar = document.querySelector("#dsSidebar")?.getBoundingClientRect();
    const mainArea = document.querySelector("#dsMainArea")?.getBoundingClientRect();
    return { sidebarRight: sidebar?.right ?? 0, mainLeft: mainArea?.left ?? 0 };
  });
  assert(
    mobileLayout.mainLeft >= mobileLayout.sidebarRight - 1,
    `admin-ds mobile main content must clear collapsed sidebar (sidebarRight=${mobileLayout.sidebarRight}, mainLeft=${mobileLayout.mainLeft})`,
  );
  await page.locator("#dsSidebarToggle").click();
  await page.waitForFunction(() => document.querySelector("#dsSidebarOverlay")?.classList.contains("show"));
  await page.locator("#dsSidebarOverlay").click();
  await page.waitForFunction(() => document.querySelector("#dsSidebar")?.classList.contains("collapsed"));
}

async function verifySceneHotspotSizes(page, baseUrl) {
  // 守护 ASSET_HANDOFF §5 四层契约：已四层化页面(index/creative/world-square)的透明命中区
  // 必须固定 64×34，禁止回灌 clamp(64px,8vw,108px) 或加大热点尺寸（2026-06-17 world-square
  // 桌面 108px 回归即由此类运行时尺寸断言捕获，静态 CSS 断言会被同选择器多处规则绕过）。
  const cases = [
    { path: "/index.html", label: "index" },
    { path: "/creative.html", label: "creative" },
    { path: "/world-square.html", label: "world-square" },
  ];
  const viewports = [
    { width: 1440, height: 900, name: "desktop" },
    { width: 390, height: 844, name: "mobile" },
  ];
  for (const vp of viewports) {
    for (const c of cases) {
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto(`${baseUrl}${c.path}?verify=frontend-realness`, { waitUntil: "networkidle" });
      const sizes = await page.evaluate(() =>
        Array.from(document.querySelectorAll(".scene-hotspot")).map((n) => ({ w: n.offsetWidth, h: n.offsetHeight })),
      );
      assert(sizes.length > 0, `${c.label} [${vp.name}] should render scene hotspots`);
      for (const s of sizes) {
        assert(s.w === 64, `${c.label} [${vp.name}] hotspot width must be fixed 64px (ASSET_HANDOFF §5), got ${s.w}`);
        assert(s.h === 34, `${c.label} [${vp.name}] hotspot height must be 34px, got ${s.h}`);
      }
    }
  }
}

async function verifyPushToggleDormant(page, baseUrl) {
  // WebPush 开关休眠合同（蓝图序 2）：静态预览/无可达网关时按钮必须保持
  // 隐藏不骚扰（fail-closed 网关解析 → unsupported）；sw.js 与 PWA 图标
  // 必须可从同源取得，且 sw.js 注册了 push 事件监听。
  await page.goto(`${baseUrl}/creative.html?verify=frontend-realness-push`, { waitUntil: "networkidle" });
  await page.waitForTimeout(300);
  const toggleHidden = await page.$eval("[data-push-toggle]", (el) => el.hidden);
  assert(toggleHidden === true, "push toggle must stay dormant without a reachable gateway");
  const swResponse = await page.request.get(`${baseUrl}/sw.js`);
  assert(swResponse.status() === 200, "sw.js must be served");
  const swBody = await swResponse.text();
  assert(swBody.includes('addEventListener("push"'), "sw.js must listen for push");
  const iconResponse = await page.request.get(`${baseUrl}/assets/icons/icon-192.png`);
  assert(iconResponse.status() === 200, "PWA icon must be served");

  // install-hint 休眠合同：无 SW/未安装/未触发的 headless 环境中，加桌引导
  // chip 必须保持隐藏（与推送钮同为"不骚扰"合同）；manifest 链接必须存在。
  const manifestLink = await page.$eval('link[rel="manifest"]', (el) => el.getAttribute("href"));
  assert(manifestLink === "./manifest.webmanifest", "manifest link must point at manifest.webmanifest");
  const hintState = await page.evaluate(() => {
    const chip = document.querySelector(".install-hint-chip");
    return { present: Boolean(chip), hidden: chip ? chip.hidden : null };
  });
  assert(hintState.present === true, "install hint chip should be mounted on chat pages");
  assert(hintState.hidden === true, "install hint chip must stay hidden without an install prompt");

  // 优雅降级：授予通知权限后程序化点击开关（无可达网关）——
  // 订阅链路必须在"网关未连接"处优雅失败：按钮回落休眠、无未捕获异常。
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.context().grantPermissions(["notifications"], { origin: baseUrl });
  await page.$eval("[data-push-toggle]", (el) => el.click());
  await page.waitForTimeout(600);
  const stillHidden = await page.$eval("[data-push-toggle]", (el) => el.hidden);
  assert(stillHidden === true, "push toggle must fall back to dormant when the push service is unreachable");
  assert(errors.length === 0, `push degrade must not throw uncaught errors: ${errors.join(" | ")}`);
}

async function verifyNoJavascriptErrors(page, baseUrl) {
  // 守护所有 shell 页面桌面+移动加载无未捕获 JS 异常（如 admin ensureConversationCallout
  // insertBefore 无 parentElement 守卫导致的 DOMException 崩溃，2026-06-17 修复）。
  const pages = [
    { path: "/index.html", label: "index" },
    { path: "/creative.html", label: "creative" },
    { path: "/world-square.html", label: "world-square" },
    { path: "/unified.html", label: "unified" },
    { path: "/admin.html", label: "admin" },
    { path: "/admin-ds.html", label: "admin-ds" },
  ];
  const viewports = [
    { width: 1440, height: 900, name: "desktop" },
    { width: 390, height: 844, name: "mobile" },
  ];
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  for (const vp of viewports) {
    for (const p of pages) {
      errors.length = 0;
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto(`${baseUrl}${p.path}?verify=frontend-realness`, { waitUntil: "networkidle" }).catch(() => {});
      await page.waitForTimeout(400);
      assert(
        errors.length === 0,
        `${p.label} [${vp.name}] must load without uncaught JS errors: ${errors.join(" | ").slice(0, 240)}`,
      );
    }
  }
}

async function verifySceneEditorAccess(page, baseUrl) {
  // 守护 P1 房间编辑器入口的访问控制（app.js applyRailVisibility）：访客
  // （identity=访客）看不到编辑器入口；已登录居民可见。
  // 注：scene-editor 的 URL 构造（activeRoomId 实时取 + token 透传，废弃残缺
  // "dm:<id>:" 前缀）依赖真实 gateway 的 init 完整流程；realness 静态服务器
  // 无 gateway，refreshFromGateway 失败会中断 init 的 href 填充，故 URL 部分由
  // shell-pages-static 源码断言守护，这里只固化访问控制的运行时行为。
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto(`${baseUrl}/index.html?verify=frontend-realness&identity=${encodeURIComponent("访客")}`, { waitUntil: "networkidle" });
  await page.waitForTimeout(300);
  const guestDisplay = await page.locator("#scene-editor-link").evaluate((node) => getComputedStyle(node).display);
  assert(guestDisplay === "none", `guest must not see scene-editor link (owner-only), got display=${guestDisplay}`);

  await page.goto(`${baseUrl}/index.html?verify=frontend-realness&identity=alice`, { waitUntil: "networkidle" });
  await page.waitForTimeout(300);
  const ownerDisplay = await page.locator("#scene-editor-link").evaluate((node) => getComputedStyle(node).display);
  assert(ownerDisplay !== "none", `logged-in resident must see scene-editor link, got display=${ownerDisplay}`);
}

async function verifyCreativeMobileRelationshipActions(page, baseUrl) {
  // 守护私宅居民关系按钮在移动抽屉里可见、可点击，避免 26px 高度或被房间按钮覆盖。
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${baseUrl}/creative.html?verify=frontend-realness&identity=alice`, { waitUntil: "networkidle" });
  await page.locator("#hud-rail-toggle").click();
  await page.evaluate(() => {
    const list = document.querySelector(".creative-resident-list");
    if (!list) return;
    const li = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "secondary mini-button resident-relationship-action";
    button.textContent = "申请好友";
    button.addEventListener("click", () => {
      document.body.dataset.relationshipButtonClicked = "1";
    });
    li.appendChild(button);
    list.appendChild(li);
  });
  const action = page.locator(".creative-resident-list .resident-relationship-action").last();
  const minHeight = await action.evaluate((node) => parseFloat(getComputedStyle(node).minHeight));
  assert(minHeight >= 34, `mobile relationship action min-height must be at least 34px, got ${minHeight}`);
  await action.click();
  assert(
    await page.evaluate(() => document.body.dataset.relationshipButtonClicked === "1"),
    "mobile relationship action should receive clicks without being covered",
  );
}

async function verifySceneEditorMobile(page, baseUrl) {
  // 守护 scene-editor 移动端可用：pointer 事件替换 mouse 后加载无 JS 错误，且窄屏
  // 切单栏布局（不再被 280px 桌面侧栏挤爆）。P1 房间编辑器移动端交互闭环。
  await page.setViewportSize({ width: 390, height: 844 });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(`${baseUrl}/scene-editor.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await page.waitForTimeout(300);
  assert(errors.length === 0, `scene-editor mobile must load without JS errors: ${errors.join(" | ").slice(0, 240)}`);
  const cols = await page.locator(".editor-shell").evaluate((node) => getComputedStyle(node).gridTemplateColumns);
  // grid-template-columns: 1fr 计算值为单 track 像素（如 "390px"）；桌面双栏计算值首列是 "280px ..."。
  assert(!cols.startsWith("280px"), `scene-editor mobile must drop the 280px sidebar column (single-column), got ${cols}`);
}

async function verifySceneEditorDayNight(page, baseUrl) {
  // 守护 scene-editor 昼夜预览切换：填入 day/night URL，点击 previewBtn 应在两时段间
  // 切换 timeOfDay 并更新 img.src，让编辑者验证两个背景的热点对齐。
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${baseUrl}/scene-editor.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await page.waitForTimeout(200);
  // 空状态引导：img 无 src 时可见（CSS :has），加载背景后自动隐藏
  assert(await page.locator("#emptyHint").evaluate((n) => getComputedStyle(n).display !== "none"), "empty hint must show when no background loaded");
  await page.locator("#dayUrl").fill("https://example.test/scene-day.png");
  await page.locator("#nightUrl").fill("https://example.test/scene-night.png");
  const initial = await page.evaluate(() => document.body.dataset.timeOfDay);
  await page.locator("#previewBtn").click();
  await page.waitForTimeout(150);
  const afterClick = await page.evaluate(() => document.body.dataset.timeOfDay);
  assert(afterClick !== initial, `preview click must toggle time-of-day, ${initial} -> ${afterClick}`);
  const imgSrc = await page.locator("#sceneImage").evaluate((node) => node.getAttribute("src") || "");
  const expected = afterClick === "day" ? "scene-day" : "scene-night";
  assert(imgSrc.includes(expected), `preview must load the toggled ${afterClick} background, got ${imgSrc}`);
  assert(await page.locator("#emptyHint").evaluate((n) => getComputedStyle(n).display === "none"), "empty hint must hide once a background is loaded");
}

async function verifySceneEditorCoordinateCanvas(page, baseUrl) {
  // 背景和热点必须共享同一 16:9 逻辑画布：移动端容器比例变化时，热点不能落在图片留白上，缩放也必须同步。
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${baseUrl}/scene-editor.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await page.locator("#dayUrl").fill("data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=");
  await page.locator("#nightUrl").fill("data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=");
  await page.locator("#previewBtn").click();
  await page.locator("#addHotspotBtn").click();
  await page.locator("#zoomIn").click();
  await page.waitForTimeout(120);
  const metrics = await page.evaluate(() => {
    const canvas = document.querySelector("#sceneCanvas");
    const image = document.querySelector("#sceneImage");
    const hotspot = document.querySelector(".editor-hotspot");
    const canvasRect = canvas?.getBoundingClientRect();
    const imageRect = image?.getBoundingClientRect();
    const hotspotRect = hotspot?.getBoundingClientRect();
    return {
      canvasParent: image?.parentElement?.id || "",
      canvasRatio: canvasRect ? canvasRect.width / canvasRect.height : 0,
      imageInsideCanvas: canvas && image ? canvas.contains(image) : false,
      hotspotInsideCanvas: canvas && hotspot ? canvas.contains(hotspot) : false,
      hotspotWithinCanvas: Boolean(canvasRect && hotspotRect &&
        hotspotRect.left >= canvasRect.left - 1 &&
        hotspotRect.top >= canvasRect.top - 1 &&
        hotspotRect.right <= canvasRect.right + 1 &&
        hotspotRect.bottom <= canvasRect.bottom + 1),
      imageTransform: image?.style.transform || "",
      canvasTransform: canvas?.style.transform || "",
    };
  });
  assert(metrics.canvasParent === "sceneCanvas", `background should be inside scene canvas: ${metrics.canvasParent}`);
  assert(metrics.imageInsideCanvas, "background image must be a child of scene canvas");
  assert(metrics.hotspotInsideCanvas, "hotspot must be a child of scene canvas");
  assert(Math.abs(metrics.canvasRatio - 16 / 9) < 0.03, `scene canvas must be 16:9, got ${metrics.canvasRatio}`);
  assert(metrics.hotspotWithinCanvas, "hotspot must remain within the logical canvas");
  assert(metrics.imageTransform === "", `image should not own zoom transform: ${metrics.imageTransform}`);
  assert(metrics.canvasTransform.includes("scale"), `scene canvas should own zoom transform: ${metrics.canvasTransform}`);
}

async function verifySceneEditorHotspotList(page, baseUrl) {
  // 守护侧栏热点列表（多热点管理 UX）：添加后列表显示 + 计数更新，点击列表项选中，× 删除后同步。
  // 页面加载不触发 renderHotspots，必须靠 addHotspotBtn 驱动才能执行列表渲染逻辑。
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${baseUrl}/scene-editor.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await page.waitForTimeout(200);
  assert(await page.locator("#hotspot-list .hl-empty").count() === 1, "hotspot list should show empty hint initially");
  assert((await page.locator("#hotspot-count").textContent()) === "0", "hotspot count should be 0 initially");
  await page.locator("#addHotspotBtn").click();
  await page.locator("#addHotspotBtn").click();
  await page.waitForTimeout(120);
  const lis = page.locator("#hotspot-list li");
  assert(await lis.count() === 2, "hotspot list should list 2 hotspots after adding twice");
  assert((await page.locator("#hotspot-count").textContent()) === "2", "hotspot count should be 2 after adding");
  await lis.nth(1).click();
  await page.waitForTimeout(100);
  assert(await lis.nth(1).evaluate((n) => n.classList.contains("selected")), "clicking a list item should mark it selected");
  await page.locator("#hotspot-list li .hl-del").first().click();
  await page.waitForTimeout(100);
  assert(await lis.count() === 1, "hotspot list should drop to 1 after deleting one");
}

async function verifySceneEditorUndoRedo(page, baseUrl) {
  // 守护 undo/redo 历史栈：添加→undo 清空→redo 恢复→undo 再清空，验证结构变更可撤销/重做。
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${baseUrl}/scene-editor.html?verify=frontend-realness`, { waitUntil: "networkidle" });
  await page.waitForTimeout(200);
  await page.locator("#addHotspotBtn").click();
  await page.waitForTimeout(100);
  assert((await page.locator("#hotspot-count").textContent()) === "1", "count should be 1 after adding");
  await page.locator("#undoBtn").click();
  await page.waitForTimeout(100);
  assert((await page.locator("#hotspot-count").textContent()) === "0", "undo should drop count to 0");
  assert(await page.locator("#hotspot-list .hl-empty").count() === 1, "undo should show empty hint");
  await page.locator("#redoBtn").click();
  await page.waitForTimeout(100);
  assert((await page.locator("#hotspot-count").textContent()) === "1", "redo should restore count to 1");
  await page.locator("#undoBtn").click();
  await page.waitForTimeout(100);
  assert((await page.locator("#hotspot-count").textContent()) === "0", "second undo should clear again");
}

const server = createStaticServer();
const address = await listen(server);
const baseUrl = `http://${address.address}:${address.port}`;
const browser = await chromium.launch({ headless: true });

try {
  const page = await browser.newPage();
  await verifyCreativeComposer(page, baseUrl);
  await verifyCreativeHotspots(page, baseUrl);
  await verifySceneRails(page, baseUrl);
  await verifyDayNightBackgrounds(page, baseUrl);
  await verifyAdminDs(page, baseUrl);
  await verifySceneHotspotSizes(page, baseUrl);
  await verifyNoJavascriptErrors(page, baseUrl);
  await verifyPushToggleDormant(page, baseUrl);
  await verifySceneEditorAccess(page, baseUrl);
  await verifyCreativeMobileRelationshipActions(page, baseUrl);
  await verifySceneEditorMobile(page, baseUrl);
  await verifySceneEditorCoordinateCanvas(page, baseUrl);
  await verifySceneEditorDayNight(page, baseUrl);
  await verifySceneEditorHotspotList(page, baseUrl);
  await verifySceneEditorUndoRedo(page, baseUrl);
  console.log("frontend realness: composer, hotspot labels, shared scene rails, day/night backgrounds, formal admin, fixed hotspot sizes (64x34), zero uncaught JS errors, scene-editor owner-only access, mobile relationship actions, mobile touch, day/night preview, hotspot list and undo/redo, push toggle dormant/graceful degrade and install hint dormant passed");
} finally {
  await browser.close();
  await close(server);
}
