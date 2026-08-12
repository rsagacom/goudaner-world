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

const CASES = [
  {
    name: "creative narrow desktop",
    path: "/creative.html",
    viewport: { width: 1259, height: 872 },
    rail: "#creative-rail",
    stage: ".creative-stage",
    expectedRailWidth: 220,
    matchStageHeightToRail: true,
  },
  {
    name: "creative wide desktop",
    path: "/creative.html",
    viewport: { width: 1560, height: 873 },
    rail: "#creative-rail",
    stage: ".creative-stage",
    expectedRailWidth: 220,
    matchStageHeightToRail: true,
  },
  {
    name: "public square desktop",
    path: "/index.html",
    viewport: { width: 1560, height: 873 },
    rail: ".public-square-rail",
    stage: ".public-square-stage",
    expectedRailWidth: 220,
    matchStageHeightToRail: true,
  },
  {
    name: "world square desktop",
    path: "/world-square.html",
    viewport: { width: 1920, height: 755 },
    rail: ".world-square-rail",
    stage: ".world-square-stage",
    expectedRailWidth: 220,
    matchStageHeightToRail: true,
  },
  {
    // 2026-08-02：移动端 hotspot 画布必须等于 contain 背景的渲染盒
    // （宽 = min(stage宽, stage高×16/9)，且自身保持 16:9），不能再按 100vh 推算。
    name: "creative mobile portrait",
    path: "/creative.html",
    viewport: { width: 390, height: 844 },
    rail: null,
    stage: ".creative-stage",
    hotspotLayer: ".scene-hotspots",
    sceneAspect: 16 / 9,
    matchStageHeightToRail: false,
  },
  {
    name: "creative desktop hotspot canvas",
    path: "/creative.html",
    viewport: { width: 1560, height: 873 },
    rail: null,
    stage: ".creative-stage",
    hotspotLayer: ".scene-hotspots",
    sceneAspect: 16 / 9,
    matchStageHeightToRail: false,
  },
];

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

function assertNear(actual, expected, tolerance, label) {
  if (Math.abs(actual - expected) > tolerance) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
}

async function measureCase(page, baseUrl, item) {
  await page.setViewportSize(item.viewport);
  await page.goto(`${baseUrl}${item.path}?verify=scene-layout`, { waitUntil: "networkidle" });
  return page.evaluate(({ rail, stage, hotspotLayer }) => {
    const box = (selector) => {
      const node = selector && document.querySelector(selector);
      if (!node) return null;
      const rect = node.getBoundingClientRect();
      return {
        width: Math.round(rect.width * 10) / 10,
        height: Math.round(rect.height * 10) / 10,
      };
    };
    return {
      rail: box(rail),
      stage: box(stage),
      // container query 单位相对容器 content-box（不含边框），与 contain 背景的
      // padding-box 基准一致；断言须用 client 尺寸而非 getBoundingClientRect。
      stageClient: (() => {
        const node = stage && document.querySelector(stage);
        return node
          ? { width: Math.round(node.clientWidth * 10) / 10, height: Math.round(node.clientHeight * 10) / 10 }
          : null;
      })(),
      hotspotLayer: box(hotspotLayer),
    };
  }, item);
}

const server = createStaticServer();
const address = await listen(server);
const baseUrl = `http://${address.address}:${address.port}`;
const browser = await chromium.launch({ headless: true });

try {
  const page = await browser.newPage();
  for (const item of CASES) {
    const result = await measureCase(page, baseUrl, item);
    if (!result.stage) {
      throw new Error(`${item.name}: missing stage element`);
    }
    if (item.rail) {
      if (!result.rail) throw new Error(`${item.name}: missing rail element`);
      assertNear(result.rail.width, item.expectedRailWidth, 1, `${item.name} rail width`);
      if (item.matchStageHeightToRail) {
        assertNear(result.stage.height, result.rail.height, 1, `${item.name} stage height`);
      }
      console.log(`${item.name}: rail ${result.rail.width}x${result.rail.height}, stage ${result.stage.width}x${result.stage.height}`);
      continue;
    }
    if (item.hotspotLayer) {
      if (!result.hotspotLayer) throw new Error(`${item.name}: missing hotspot layer`);
      // 热点画布 = contain 背景渲染盒：宽 = min(stage内容宽, stage内容高×aspect)，且自身保持 aspect
      const expectedWidth = Math.min(result.stageClient.width, result.stageClient.height * item.sceneAspect);
      assertNear(result.hotspotLayer.width, expectedWidth, 1.5, `${item.name} hotspot canvas width`);
      assertNear(result.hotspotLayer.width / result.hotspotLayer.height, item.sceneAspect, 0.02, `${item.name} hotspot canvas aspect`);
      console.log(`${item.name}: stage ${result.stage.width}x${result.stage.height}, hotspot canvas ${result.hotspotLayer.width}x${result.hotspotLayer.height}`);
    }
  }
} finally {
  await browser.close();
  await close(server);
}
