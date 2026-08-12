import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

import { loadUserShellApp } from "./fake-dom.mjs";

const serial = { concurrency: false };
const TEST_SESSION_TOKEN = "lbst_test_session_token";

async function flushAsyncWork() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test("creative resident shell boots into the chat workspace with room scene and portrait chrome", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document } = app;
    const activeRoom = document.querySelector(".room-button.active");
    const roomStageCanvas = document.querySelector("#room-stage-canvas");
    const portraitCanvas = document.querySelector("#room-stage-portrait-canvas");
    const roomStageSide = document.querySelector(".conversation-stage-side");
    const chatDetailPanel = document.querySelector(".chat-detail");
    const chatDetailContent = document.querySelector("#chat-detail-content");
    const summaryTitle = document.querySelector("#chat-detail-summary-title");
    const summaryCopy = document.querySelector("#chat-detail-summary-copy");
    const cardShell = document.querySelector("#chat-detail-card-shell");
    const cardTitle = document.querySelector("#chat-detail-card-title");
    const cardAvatar = document.querySelector("#chat-detail-card-avatar");
    const cardMeta = document.querySelector("#chat-detail-card-meta");
    const cardActions = document.querySelector("#chat-detail-card-actions");
    const composerInput = document.querySelector("#composer-input");
    const composerTip = document.querySelector(".composer-tip");

    assert.equal(document.body.dataset.shellMode, "user");
    assert.equal(document.body.dataset.workspace, "chat");
    assert.equal(document.body.dataset.chatDetailMode, "inline");
    assert.equal(document.body.dataset.roomVariant, "home");
    assert.equal(document.body.dataset.roomMotif, "courtyard");
    assert.equal(document.title, "我和狗蛋儿的家 · 房间聊天");
    assert.equal(document.querySelector(".workspace-switcher"), null);
    assert.ok(activeRoom);
    assert.equal(roomStageCanvas?.dataset?.variant, "home");
    assert.equal(roomStageCanvas?.dataset?.kind, "stage");
    assert.equal(roomStageCanvas?.dataset?.motif, "courtyard");
    assert.equal(portraitCanvas?.dataset?.variant, "home");
    assert.equal(portraitCanvas?.dataset?.kind, "portrait");
    assert.equal(portraitCanvas?.dataset?.motif, "caretaker");
    assert.equal(portraitCanvas?.dataset?.monogram, "旺");
    assert.ok(roomStageSide);
    assert.ok(chatDetailPanel);
    assert.ok(chatDetailContent);
    assert.equal(chatDetailPanel?.dataset?.roomVariant, "home");
    assert.equal(chatDetailPanel?.dataset?.roomMotif, "courtyard");
    assert.equal(cardShell?.dataset?.roomVariant, "home");
    assert.equal(cardShell?.dataset?.roomMotif, "courtyard");
    assert.match(document.querySelector("#masthead-title")?.textContent || "", /住宅私聊|房内聊天/);
    assert.match(document.querySelector("#hero-note")?.textContent || "", /住处|一对一|房间里继续聊/);
    assert.match(summaryTitle?.textContent || "", /住宅私聊|房内状态/);
    assert.match(summaryCopy?.textContent || "", /续聊|记任务|追问|一对一/);
    assert.match(cardTitle?.textContent || "", /房内状态|住宅私聊|角色卡/);
    assert.match(cardAvatar?.textContent || "", /旺|房|住/);
    assert.match(cardMeta?.textContent || "", /住户|同住AI|状态|自动回复/);
    assert.match(cardActions?.textContent || "", /续聊/);
    assert.match(cardActions?.textContent || "", /整理/);
    assert.match(cardActions?.textContent || "", /留条/);
    assert.match(document.querySelector(".conversation-stage-note")?.textContent || "", /适合快速确认方向/);
    assert.match(roomStageSide.textContent || "", /旺财|房间管家|直接协作/);
    assert.match(chatDetailContent.textContent || "", /旺财|自动回复|内测同伴/);

    const continueButton = document.querySelector('[data-card-action="续聊"]');
    assert.ok(continueButton);
    continueButton.click();

    assert.equal(composerInput?.value, "续聊：");
    assert.equal(continueButton.classList.contains("is-active"), true);
    assert.match(composerTip?.textContent || "", /当前动作 续聊/);
  } finally {
    app.cleanup();
  }
});

test("creative resident shell opens with the first room selected and composer enabled", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document } = app;
    const composerInput = document.querySelector("#composer-input");
    const roomButtons = document.querySelectorAll(".room-button");
    const activeRoom = document.querySelector(".room-button.active");

    assert.ok(activeRoom);
    assert.equal(activeRoom, roomButtons[0]);
    assert.equal(composerInput?.disabled, false);
    assert.ok((composerInput?.placeholder || "").length > 0);
  } finally {
    app.cleanup();
  }
});

test("creative resident login starts in request step and switches to verify after OTP request", serial, async () => {
  const html = await fs.readFile(new URL("../creative.html", import.meta.url), "utf8");
  assert.match(html, /id="auth-verify-form"[^>]*resident-login-verify[^>]*shell-hidden/);

  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document } = app;
    const requestForm = document.querySelector("#auth-request-form");
    const verifyForm = document.querySelector("#auth-verify-form");
    const emailInput = document.querySelector("#auth-email-input");

    assert.equal(verifyForm?.classList.contains("shell-hidden"), true);
    assert.equal(requestForm?.dataset.authStep, "request");

    emailInput.value = "qa@example.com";
    requestForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(requestForm?.dataset.authStep, "verify");
    assert.equal(verifyForm?.classList.contains("shell-hidden"), false);
    assert.equal(emailInput.disabled, true);
    assert.match(document.querySelector("#auth-status")?.textContent || "", /验证码已发往/);
  } finally {
    app.cleanup();
  }
});

test("creative resident composer auto-resizes to five-line cap", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document } = app;
    const composerInput = document.querySelector("#composer-input");

    composerInput.scrollHeight = 260;
    composerInput.value = "第一行\n第二行\n第三行\n第四行\n第五行\n第六行";
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));

    assert.equal(composerInput.style.height, "120px");
    assert.equal(composerInput.style.overflowY, "auto");
  } finally {
    app.cleanup();
  }
});

test("creative resident timeline follows only when user is already near bottom", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const timeline = document.querySelector("#timeline");
    const roomButtons = document.querySelectorAll(".room-button");

    timeline.scrollHeight = 1200;
    timeline.clientHeight = 320;
    timeline.scrollTop = 160;
    roomButtons[1].click();
    await flushAsyncWork();
    assert.equal(timeline.scrollTop, 160);

    timeline.scrollHeight = 1200;
    timeline.clientHeight = 320;
    timeline.scrollTop = 880;
    roomButtons[0].click();
    await flushAsyncWork();
    assert.equal(timeline.scrollTop, 1200);
  } finally {
    app.cleanup();
  }
});

test("creative resident empty conversation keeps scene clear instead of skeleton bubbles", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const builderRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /builder/.test(node.textContent || ""),
    );

    assert.ok(builderRoom);
    builderRoom.click();
    await flushAsyncWork();

    const skeletonRows = document.querySelectorAll(".message-row.timeline-skeleton-row");
    const timeline = document.querySelector("#timeline");
    assert.equal(skeletonRows.length, 0);
    assert.equal(document.querySelector(".timeline-empty"), null);
    assert.equal(timeline?.querySelector(".timeline-skeleton-bubble"), null);
    assert.equal(timeline?.querySelector(".message"), null);
    assert.equal(timeline?.children.length, 0);
  } finally {
    app.cleanup();
  }
});

test("creative shell CSS keeps pixel scene, hotspots and mobile composer in one safe coordinate system", serial, async () => {
  const pixelCss = await fs.readFile(new URL("../styles.pixel-map.css", import.meta.url), "utf8");
  const creativeCss = await fs.readFile(new URL("../styles.creative.css", import.meta.url), "utf8");

  assert.match(pixelCss, /--creative-scene-aspect:\s*16\s*\/\s*9/);
  assert.match(pixelCss, /body\[data-shell-variant="creative-terminal"\]\s+\.creative-stage[\s\S]*aspect-ratio:\s*var\(--creative-scene-aspect\)/);
  assert.match(pixelCss, /body\[data-shell-variant="creative-terminal"\]\s+\.creative-stage[\s\S]*background:[\s\S]*contain\s+no-repeat/);
  assert.match(pixelCss, /body\[data-shell-variant="creative-terminal"\]\s+\.scene-hotspots[\s\S]*aspect-ratio:\s*var\(--creative-scene-aspect\)/);
  assert.match(pixelCss, /env\(safe-area-inset-bottom\)/);
  assert.match(creativeCss, /\.creative-composer textarea[\s\S]*max-height:\s*120px/);
  const composerRule = pixelCss.match(/body\[data-shell-page="hub"\]\[data-shell-variant="public-square"\]\s+\.public-square-composer textarea,\s*body\[data-shell-variant="creative-terminal"\]\s+\.creative-composer textarea\s*\{[^}]*\}/)?.[0] ?? "";
  assert.ok(composerRule, "expected creative composer textarea override");
  assert.doesNotMatch(composerRule, /height:\s*34px\s*!important/);
});

test("creative resident shell can refresh gateway badges without provider controls on the chat page", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document } = app;
    assert.match(document.querySelector("#gateway-state")?.textContent || "", /127\.0\.0\.1:50651/);
    assert.match(document.querySelector("#provider-state")?.textContent || "", /cloudflare\.com|消息来源/);
    assert.ok(document.querySelector(".room-button.active"));
  } finally {
    app.cleanup();
  }
});

test("creative resident gateway offline state disables composer and marks the shell offline", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "rsaga",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
    gatewayProviderState: {
      mode: "cloudflare",
      reachable: false,
      connection_state: "Disconnected",
      base_url: "https://cloudflare.com/fake-provider",
    },
  });
  try {
    const { document } = app;
    const composerInput = document.querySelector("#composer-input");
    const composerSend = document.querySelector("#composer-send");

    assert.equal(document.body.dataset.gatewayConnection, "offline");
    assert.match(document.querySelector("#provider-state")?.textContent || "", /已断开|降级/);
    assert.equal(composerInput?.disabled, true);
    assert.equal(composerSend?.disabled, true);
    assert.match(composerInput?.placeholder || "", /离线|同步恢复/);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident shell scopes state and opens SSE stream by stored resident identity", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "rsaga",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, fetchCalls, eventSourceCalls } = app;
    const composerInput = document.querySelector("#composer-input");

    assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/shell/state?resident_id=rsaga"));
    assert.ok(eventSourceCalls.includes("http://127.0.0.1:50651/v1/shell/events?resident_id=rsaga"));
    assert.equal(composerInput?.disabled, false);
    assert.equal(document.querySelector("#resident-login-card")?.classList.contains("shell-hidden"), true);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident shell clears an expired session on shell-state 401", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    gatewayShellStateShouldFail: true,
    gatewayShellStateFailureStatus: 401,
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });

  try {
    const { document, window } = app;
    assert.equal(window.localStorage.getItem("lobster-session-token"), "");
    assert.equal(document.body.dataset.gatewayConnection, "offline");
    assert.match(document.querySelector("#auth-status")?.textContent || "", /登录已失效，请重新登录/);
    assert.equal(document.querySelector("#composer-input")?.disabled, true);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident SSE reopens with state version wait cursor", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "rsaga",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { eventSourceCalls, emitEventSource } = app;
    assert.equal(eventSourceCalls.at(-1), "http://127.0.0.1:50651/v1/shell/events?resident_id=rsaga");

    const payload = JSON.parse(
      await fs.readFile(new URL("../generated/state.contract.json", import.meta.url), "utf8"),
    );
    payload.state_version = "shell:v1:test";
    emitEventSource("shell-state", payload);
    await new Promise((resolve) => setTimeout(resolve, 20));

    assert.equal(
      eventSourceCalls.at(-1),
      "http://127.0.0.1:50651/v1/shell/events?resident_id=rsaga&after=shell%3Av1%3Atest&wait_ms=4000",
    );
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident send clears composer and posts once", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa2",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, fetchCalls } = app;
    const composerForm = document.querySelector("#composer");
    const composerInput = document.querySelector("#composer-input");
    assert.ok(composerForm);
    assert.ok(composerInput);
    assert.equal(composerInput.disabled, false);

    composerInput.value = "你好";
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));
    composerForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(composerInput.value, "");
    assert.equal(
      fetchCalls.filter((url) => url === "http://127.0.0.1:50651/v1/shell/message").length,
      1,
    );
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident Enter key sends and Shift Enter keeps a draft", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa2",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, fetchCalls } = app;
    const composerInput = document.querySelector("#composer-input");
    assert.ok(composerInput);
    assert.equal(composerInput.disabled, false);

    composerInput.value = "换行草稿";
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));
    composerInput.dispatchEvent(new Event("keydown", { key: "Enter", shiftKey: true, bubbles: true, cancelable: true }));
    await flushAsyncWork();
    assert.equal(
      fetchCalls.filter((url) => url === "http://127.0.0.1:50651/v1/shell/message").length,
      0,
    );
    assert.equal(composerInput.value, "换行草稿");

    composerInput.dispatchEvent(new Event("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(composerInput.value, "");
    assert.equal(
      fetchCalls.filter((url) => url === "http://127.0.0.1:50651/v1/shell/message").length,
      1,
    );
  } finally {
    app.cleanup();
  }
});

test("Enter sends on scene composer regardless of viewport width", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa2",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, fetchCalls } = app;
    const composerForm = document.querySelector("#composer");
    const composerInput = document.querySelector("#composer-input");
    assert.ok(composerForm);
    assert.ok(composerInput);

    // Ensure the composer has a scene-composer class
    composerForm.classList.add("public-square-composer");

    composerInput.value = "移动端发送测试";
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));
    composerInput.dispatchEvent(new Event("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(composerInput.value, "");
    assert.equal(
      fetchCalls.filter((url) => url === "http://127.0.0.1:50651/v1/shell/message").length,
      1,
    );
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident send commits one stable self bubble with avatar", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651&identity=qa-a&qa=browser",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa-a",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, fetchCalls } = app;
    const composerForm = document.querySelector("#composer");
    const composerInput = document.querySelector("#composer-input");
    const text = `双端回归-${Date.now()}`;

    assert.ok(composerForm);
    assert.ok(composerInput);
    assert.equal(composerInput.disabled, false);

    composerInput.value = text;
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));
    composerForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    for (let index = 0; index < 6; index += 1) {
      await flushAsyncWork();
    }

    const matchingRows = Array.from(document.querySelectorAll(".message-row")).filter((row) =>
      (row.textContent || "").includes(text),
    );

    assert.equal(composerInput.value, "");
    assert.equal(
      fetchCalls.filter((url) => url === "http://127.0.0.1:50651/v1/shell/message").length,
      1,
    );
    assert.equal(matchingRows.length, 1);
    assert.equal(matchingRows[0]?.classList.contains("self"), true);
    assert.equal(matchingRows[0]?.dataset?.messageKind, "self");
    assert.ok(matchingRows[0]?.querySelector(".message-avatar"));
    assert.equal(matchingRows[0]?.querySelector(".message-pending"), null);
    assert.doesNotMatch(matchingRows[0]?.textContent || "", /待同步|正在投递|发送失败|待重发/);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident export surfaces gateway Error message", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651&identity=rsaga&qa=browser",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "rsaga",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
    exportResponse: {
      status: 403,
      body: {
        Error: {
          message: "导出权限不足",
        },
      },
    },
  });
  try {
    const { document, fetchCalls } = app;
    const exportButton = Array.from(document.querySelectorAll("button")).find((node) =>
      /导出当前|导出聊天|导出会话/.test(node.textContent || ""),
    );
    assert.ok(exportButton);
    assert.equal(exportButton.disabled, false);

    exportButton.click();
    await flushAsyncWork();
    await flushAsyncWork();

    assert.ok(fetchCalls.some((url) => url.startsWith("http://127.0.0.1:50651/v1/export?")));
    assert.match(document.querySelector("#world-state")?.textContent || "", /导出权限不足/);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident peer message renders on the left with its own avatar", serial, async () => {
  const baseFixtureUrl = new URL("../generated/state.contract.json", import.meta.url);
  const tempFixtureName = `state.contract.peer-message-${process.pid}-${Date.now()}.json`;
  const tempFixtureUrl = new URL(`../generated/${tempFixtureName}`, import.meta.url);
  const payload = JSON.parse(await fs.readFile(baseFixtureUrl, "utf8"));
  const conversation = payload?.conversation_shell?.conversations?.find(
    (item) => item?.conversation_id === "room:world:lobby",
  );
  const text = `对端消息-${Date.now()}`;

  assert.ok(conversation);
  conversation.messages = [
    ...(Array.isArray(conversation.messages) ? conversation.messages : []),
    {
      message_id: "msg:test-peer-visible",
      sender: "qa-a",
      timestamp_ms: Date.now(),
      timestamp_label: "刚刚",
      timestamp: "刚刚",
      text,
      delivery_status: "delivered",
    },
  ];
  await fs.writeFile(tempFixtureUrl, JSON.stringify(payload, null, 2), "utf8");

  try {
    const app = await loadUserShellApp({
      useGeneratedFixtures: true,
      generatedShellFixture: `generated/${tempFixtureName}`,
      locationSearch: "?gateway=http://127.0.0.1:50651&identity=qa-b&qa=browser",
      gatewayBaseUrl: "http://127.0.0.1:50651",
      localStorageEntries: {
        "lobster-identity": "qa-b",
        "lobster-session-token": TEST_SESSION_TOKEN,
      },
    });
    try {
      const { document, fetchCalls, eventSourceCalls } = app;
      const matchingRows = Array.from(document.querySelectorAll(".message-row")).filter((row) =>
        (row.textContent || "").includes(text),
      );

      assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/shell/state?resident_id=qa-b"));
      assert.ok(eventSourceCalls.includes("http://127.0.0.1:50651/v1/shell/events?resident_id=qa-b"));
      assert.equal(matchingRows.length, 1);
      assert.equal(matchingRows[0]?.classList.contains("self"), false);
      assert.equal(matchingRows[0]?.dataset?.messageSide, "peer");
      assert.ok(matchingRows[0]?.querySelector(".message-avatar"));
      assert.match(matchingRows[0]?.querySelector(".message-avatar")?.textContent || "", /QA|聊/);
    } finally {
      app.cleanup();
    }
  } finally {
    await fs.unlink(tempFixtureUrl).catch(() => {});
  }
});

test("gateway creative resident shell keeps visitor scoped and blocks sending before login", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document, fetchCalls, eventSourceCalls } = app;
    const composerInput = document.querySelector("#composer-input");
    const composerSend = document.querySelector("#composer-send");
    const loginCard = document.querySelector("#resident-login-card");

    assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/shell/state?resident_id=%E8%AE%BF%E5%AE%A2"));
    assert.ok(eventSourceCalls.includes("http://127.0.0.1:50651/v1/shell/events?resident_id=%E8%AE%BF%E5%AE%A2"));
    assert.equal(composerInput?.disabled, true);
    assert.equal(composerSend?.disabled, true);
    assert.match(composerInput?.placeholder || "", /请先登录后发送/);
    assert.equal(loginCard?.classList.contains("shell-hidden"), false);
  } finally {
    app.cleanup();
  }
});

test("gateway creative shell ignores an unauthenticated query identity and keeps visitor sending disabled", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651&identity=qa-visitor",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document, fetchCalls, eventSourceCalls } = app;
    const composerInput = document.querySelector("#composer-input");
    const composerSend = document.querySelector("#composer-send");

    assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/shell/state?resident_id=%E8%AE%BF%E5%AE%A2"));
    assert.ok(eventSourceCalls.includes("http://127.0.0.1:50651/v1/shell/events?resident_id=%E8%AE%BF%E5%AE%A2"));
    assert.equal(composerInput?.disabled, true);
    assert.equal(composerSend?.disabled, true);
    assert.match(composerInput?.placeholder || "", /请先登录后发送/);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident login makes email OTP the explicit default path", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document } = app;
    const deliverySelect = document.querySelector("#auth-delivery-select");
    const residentInput = document.querySelector("#auth-resident-input");
    const emailInput = document.querySelector("#auth-email-input");
    const mobileInput = document.querySelector("#auth-mobile-input");
    const deviceInput = document.querySelector("#auth-device-input");
    const challengeInput = document.querySelector("#auth-challenge-input");
    const requestButton = document.querySelector("#auth-request-button");
    const verifyForm = document.querySelector("#auth-verify-form");

    assert.equal(deliverySelect?.value, "email");
    assert.match(residentInput?.placeholder || "", /居民名/);
    assert.match(deliverySelect?.textContent || "", /邮箱验证码/);
    assert.match(emailInput?.placeholder || "", /接收验证码/);
    assert.match(mobileInput?.placeholder || "", /反滥用/);
    assert.match(deviceInput?.placeholder || "", /反滥用/);
    assert.match(requestButton?.textContent || "", /登录 \/ 注册/);
    assert.equal(challengeInput?.getAttribute("type"), "hidden");
    assert.doesNotMatch(verifyForm?.textContent || "", /challenge id|挑战标识/i);
    assert.match(verifyForm?.textContent || "", /验证码来自上一步邮件/);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident shell refreshes resident conversations after OTP login", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document, window, fetchCalls } = app;
    const challengeInput = document.querySelector("#auth-challenge-input");
    const residentInput = document.querySelector("#auth-resident-input");
    const codeInput = document.querySelector("#auth-code-input");
    const verifyForm = document.querySelector("#auth-verify-form");

    challengeInput.value = "otp:test";
    residentInput.value = "rsaga";
    codeInput.value = "123456";
    verifyForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(window.localStorage.getItem("lobster-identity"), "rsaga");
    assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/auth/email-otp/verify"));
    assert.ok(fetchCalls.includes("http://127.0.0.1:50651/v1/shell/state?resident_id=rsaga"));
    assert.equal(document.querySelector("#resident-login-card")?.classList.contains("shell-hidden"), true);
    assert.equal(document.querySelector("#composer-input")?.disabled, false);
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident sends bearer session token after OTP login", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document, window, fetchRequests } = app;
    const challengeInput = document.querySelector("#auth-challenge-input");
    const residentInput = document.querySelector("#auth-resident-input");
    const codeInput = document.querySelector("#auth-code-input");
    const verifyForm = document.querySelector("#auth-verify-form");
    const composerForm = document.querySelector("#composer");
    const composerInput = document.querySelector("#composer-input");

    challengeInput.value = "otp:test";
    residentInput.value = "rsaga";
    codeInput.value = "123456";
    verifyForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(window.localStorage.getItem("lobster-session-token"), "lbst_test_session_token");

    composerInput.value = "带 token 发送";
    composerInput.dispatchEvent(new Event("input", { bubbles: true }));
    composerForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    const messageRequest = fetchRequests.find(
      (request) => request.url === "http://127.0.0.1:50651/v1/shell/message",
    );
    assert.equal(messageRequest?.init?.headers?.Authorization, "Bearer lbst_test_session_token");
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident export sends bearer session token after OTP login", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });
  try {
    const { document, fetchRequests } = app;
    const challengeInput = document.querySelector("#auth-challenge-input");
    const residentInput = document.querySelector("#auth-resident-input");
    const codeInput = document.querySelector("#auth-code-input");
    const verifyForm = document.querySelector("#auth-verify-form");

    challengeInput.value = "otp:test";
    residentInput.value = "rsaga";
    codeInput.value = "123456";
    verifyForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();

    const exportButton = Array.from(document.querySelectorAll("button")).find((node) =>
      /导出当前|导出聊天|导出会话/.test(node.textContent || ""),
    );
    assert.ok(exportButton);
    exportButton.click();
    await flushAsyncWork();
    await flushAsyncWork();

    const exportRequest = fetchRequests.find((request) =>
      request.url.startsWith("http://127.0.0.1:50651/v1/export?"),
    );
    assert.equal(exportRequest?.init?.headers?.Authorization, "Bearer lbst_test_session_token");
  } finally {
    app.cleanup();
  }
});

test("gateway creative resident export clears session when bearer token is rejected", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    exportResponse: {
      status: 401,
      body: {
        Error: {
          message: "authorization bearer token required",
        },
      },
    },
  });
  try {
    const { document, window } = app;
    const challengeInput = document.querySelector("#auth-challenge-input");
    const residentInput = document.querySelector("#auth-resident-input");
    const codeInput = document.querySelector("#auth-code-input");
    const verifyForm = document.querySelector("#auth-verify-form");

    challengeInput.value = "otp:test";
    residentInput.value = "rsaga";
    codeInput.value = "123456";
    verifyForm.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    await flushAsyncWork();
    await flushAsyncWork();
    assert.equal(window.localStorage.getItem("lobster-session-token"), "lbst_test_session_token");

    const exportButton = Array.from(document.querySelectorAll("button")).find((node) =>
      /导出当前|导出聊天|导出会话/.test(node.textContent || ""),
    );
    assert.ok(exportButton);
    exportButton.click();
    await flushAsyncWork();
    await flushAsyncWork();

    assert.equal(window.localStorage.getItem("lobster-session-token"), "");
    assert.match(document.querySelector("#auth-status")?.textContent || "", /登录已失效，请重新登录/);
  } finally {
    app.cleanup();
  }
});

test("creative resident shell can hydrate from conversation_shell and scene_render without legacy rooms", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const activeRoom = document.querySelector(".room-button.active");
    const roomStageTitle = document.querySelector("#room-stage-title");
    const roomStageNote = document.querySelector(".conversation-stage-note");
    const portraitCanvas = document.querySelector("#room-stage-portrait-canvas");
    const summaryTitle = document.querySelector("#chat-detail-summary-title");
    const summaryCopy = document.querySelector("#chat-detail-summary-copy");
    const mastheadTitle = document.querySelector("#masthead-title");
    const roomStageSide = document.querySelector(".conversation-stage-side");
    const cardTitle = document.querySelector("#chat-detail-card-title");
    const cardMeta = document.querySelector("#chat-detail-card-meta");
    const cardActions = document.querySelector("#chat-detail-card-actions");
    const chatDetailContent = document.querySelector("#chat-detail-content");
    const cardWorkflowButton = Array.from(cardActions?.querySelectorAll("button") || []).find(
      (node) => /合同跟进委托|跟进委托/.test(node.textContent || ""),
    );
    const cardAdvanceButton = Array.from(cardActions?.querySelectorAll("button") || []).find(
      (node) => /合同标记已回执|标记已回执|已回执/.test(node.textContent || ""),
    );
    const activeRoomInlineActions = activeRoom?.querySelector(".room-inline-actions");
    const activeRoomInlinePrimary = activeRoomInlineActions?.querySelector(
      '.room-inline-action[data-room-inline-role="primary"]',
    );
    const activeRoomInlineSecondary = activeRoomInlineActions?.querySelector(
      '.room-inline-action[data-room-inline-role="secondary"]',
    );
    const conversationOverview = document.querySelector(".conversation-overview");
    const overviewProgress = conversationOverview?.querySelector(".workflow-progress");
    const overviewActions = conversationOverview?.querySelector(".overview-actions");
    const overviewActionButton = Array.from(overviewActions?.querySelectorAll("button") || []).find(
      (node) => /合同跟进委托|跟进委托/.test(node.textContent || ""),
    );
    const gatewayHotspots = Array.from(document.querySelectorAll(".scene-hotspot[data-hotspot-source='gateway']"));
    const creativeStage = document.querySelector(".creative-stage, .user-stage");
    const creativeStageImage = () =>
      typeof creativeStage?.style?.getPropertyValue === "function"
        ? creativeStage.style.getPropertyValue("--creative-scene-image")
        : creativeStage?.style?.["--creative-scene-image"] || "";

    assert.ok(activeRoom);
    assert.match(activeRoom.textContent || "", /合同线程标题 · 广场群聊/);
    assert.match(activeRoom.textContent || "", /合同广场/);
    assert.match(activeRoom.querySelector(".room-kicker")?.textContent || "", /跨城共响回廊/);
    assert.match(activeRoom.querySelector(".room-sub")?.textContent || "", /合同列表摘要 · 3 人 · 2 条消息/);
    assert.match(activeRoom.querySelector(".room-status-line")?.textContent || "", /合同状态行 · 回廊直连/);
    assert.match(summaryTitle?.textContent || "", /合同广场/);
    assert.match(summaryCopy?.textContent || "", /合同字段已经可以直接驱动/);
    assert.match(mastheadTitle?.textContent || "", /公共频道|群聊现场/);
    assert.match(roomStageTitle?.textContent || "", /合同回廊主舞台/);
    assert.match(roomStageNote?.textContent || "", /直接来自 scene_render 合同/);
    assert.equal(portraitCanvas?.dataset?.monogram, "合");
    assert.match(roomStageSide?.textContent || "", /合同巡视犬/);
    assert.match(roomStageSide?.textContent || "", /合同角色卡/);
    assert.match(roomStageSide?.textContent || "", /在线巡视 · 回廊值守/);
    assert.match(cardTitle?.textContent || "", /合同巡视犬/);
    assert.match(cardMeta?.textContent || "", /频道巡视|在线巡视/);
    assert.match(cardActions?.textContent || "", /私聊/);
    assert.match(cardActions?.textContent || "", /委托/);
    assert.match(cardActions?.textContent || "", /交易/);
    assert.match(chatDetailContent?.textContent || "", /自动回复|短期记忆|回廊/);
    assert.ok(activeRoomInlineActions);
    assert.match(activeRoomInlinePrimary?.textContent || "", /合同跟进委托/);
    assert.match(activeRoomInlineSecondary?.textContent || "", /合同标记已回执/);
    assert.ok(cardWorkflowButton);
    assert.equal(cardWorkflowButton?.textContent, "合同跟进委托");
    assert.ok(cardAdvanceButton);
    assert.equal(cardAdvanceButton?.textContent, "合同标记已回执");
    assert.ok(overviewActionButton);
    assert.equal(overviewActionButton?.textContent, "合同跟进委托");
    assert.match(overviewProgress?.textContent || "", /待回执/);
    assert.match(overviewProgress?.textContent || "", /已回执/);
    assert.equal(gatewayHotspots.length, 1);
    assert.equal(gatewayHotspots[0]?.dataset?.hotspotId, "notice-board");
    assert.match(gatewayHotspots[0]?.textContent || "", /公告牌/);
    assert.equal(gatewayHotspots[0]?.style?.left, "72%");
    assert.equal(gatewayHotspots[0]?.style?.top, "22%");
    assert.equal(creativeStage?.dataset?.imageLayerId, "image-layer");
    assert.equal(creativeStage?.dataset?.imageLayerPreset, "contract-square-night");
    assert.match(
      creativeStageImage(),
      /hub-main-city-scene-v1(?:-day)?-256\.png/,
    );

    const directRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /居所 · builder|居所直聊/.test(node.textContent || ""),
    );
    directRoom?.click();
    assert.equal(creativeStage?.dataset?.imageLayerPreset, "contract-private-room");
    assert.match(
      creativeStageImage(),
      /creative-room-scene-v2(?:-day)?-256\.png/,
    );
  } finally {
    app.cleanup();
  }
});

test("contract scene render carries gateway-owned image and hotspot layers", async () => {
  const payload = JSON.parse(
    await fs.readFile(new URL("../generated/state.contract.json", import.meta.url), "utf8"),
  );
  const scenes = payload?.scene_render?.scenes || [];
  const publicScene = scenes.find((scene) => scene.conversation_id === "room:world:lobby");
  const directScene = scenes.find((scene) => scene.conversation_id === "dm:builder:rsaga");

  assert.equal(publicScene?.image_layer?.layer_id, "image-layer");
  assert.equal(publicScene?.hotspot_layer?.coordinate_system, "scene-permyriad");
  assert.equal(publicScene?.hotspot_layer?.hotspots?.[0]?.label, "公告牌");
  assert.equal(directScene?.image_layer?.preset, "contract-private-room");
  assert.equal(directScene?.hotspot_layer?.hotspots?.[0]?.label, "工作台");
});

test("contract action templates override local defaults for card and workflow actions", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const composerInput = document.querySelector("#composer-input");
    const composerSend = document.querySelector("#composer-send");
    const entrustButton = document.querySelector('[data-card-action="委托"]');

    assert.ok(composerInput);
    assert.ok(composerSend);
    assert.ok(entrustButton);

    entrustButton.click();
    await flushAsyncWork();

    assert.equal(
      composerInput?.value,
      ["合同委托：", "- 合同需求：", "- 合同截止：", "- 合同交付："].join("\n"),
    );
    assert.equal(composerSend?.textContent, "合同发单");
    assert.notEqual(composerSend?.textContent, "发出委托");
    assert.doesNotMatch(composerInput?.value || "", /- 需求：/);

    const repliedStage = Array.from(
      document.querySelectorAll(".workflow-progress-step"),
    ).find((node) => node?.dataset?.stageLabel === "已回执" || /已回执/.test(node?.textContent || ""));
    assert.ok(repliedStage);

    repliedStage.dispatchEvent(new Event("click", { bubbles: true, cancelable: true }));
    await flushAsyncWork();

    assert.equal(
      composerInput?.value,
      ["合同委托回执：", "- 合同回执：", "- 合同待确认：", "- 合同下一步："].join("\n"),
    );
    assert.doesNotMatch(composerInput?.value || "", /- 回执：/);
  } finally {
    app.cleanup();
  }
});

test("contract hydration ignores legacy-only rooms when conversation_shell exists", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract-legacy-shadow.json",
  });
  try {
    const { document } = app;
    const roomButtons = Array.from(document.querySelectorAll(".room-button"));

    assert.equal(roomButtons.length, 1);
    assert.match(roomButtons[0]?.textContent || "", /合同入口广场/);
    assert.match(document.querySelector(".room-button.active")?.textContent || "", /合同入口广场/);
    assert.equal(document.body.textContent.includes("旧影子会话"), false);
  } finally {
    app.cleanup();
  }
});

test("creative resident shell public detail-card fallback prefers thread headline over raw room title", serial, async () => {
  const baseFixtureUrl = new URL("../generated/state.contract.json", import.meta.url);
  const tempFixtureName = `state.contract.public-fallback-${process.pid}-${Date.now()}.json`;
  const tempFixtureUrl = new URL(`../generated/${tempFixtureName}`, import.meta.url);
  const payload = JSON.parse(await fs.readFile(baseFixtureUrl, "utf8"));
  const publicConversation = payload?.conversation_shell?.conversations?.find((item) =>
    /广场群聊/.test(item?.thread_headline || ""),
  );

  assert.ok(publicConversation);
  publicConversation.detail_card = null;
  publicConversation.caretaker = null;
  await fs.writeFile(tempFixtureUrl, JSON.stringify(payload, null, 2), "utf8");

  try {
    const app = await loadUserShellApp({
      useGeneratedFixtures: true,
      generatedShellFixture: `generated/${tempFixtureName}`,
    });
    try {
      const { document } = app;
      const cardTitle = document.querySelector("#chat-detail-card-title");
      const cardMeta = document.querySelector("#chat-detail-card-meta");

      assert.match(cardTitle?.textContent || "", /公共频道 \/ 当前状态/);
      assert.match(cardMeta?.textContent || "", /合同线程标题 · 广场群聊/);
      assert.doesNotMatch(cardMeta?.textContent || "", /合同入口广场/);
    } finally {
      app.cleanup();
    }
  } finally {
    await fs.unlink(tempFixtureUrl).catch(() => {});
  }
});

test("contract thread headline can drive overview when overview summary is absent", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const builderRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /builder/.test(node.textContent || ""),
    );

    assert.ok(builderRoom);
    builderRoom.click();
    await flushAsyncWork();

    const overviewSummaries = Array.from(
      document.querySelectorAll(".overview-summary"),
    ).map((node) => node.textContent || "");

    assert.ok(
      overviewSummaries.some((text) => /合同线程标题 · 居所直聊/.test(text)),
    );
  } finally {
    app.cleanup();
  }
});

test("contract preview and activity labels can drive room button copy without legacy messages", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
  });
  try {
    const { document } = app;
    const builderRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /builder/.test(node.textContent || ""),
    );

    assert.ok(builderRoom);
    assert.match(builderRoom.textContent || "", /合同预览摘要 · 适合直接继续一对一沟通/);
    assert.match(builderRoom.textContent || "", /暂无消息/);
  } finally {
    app.cleanup();
  }
});

test("creative resident rail exposes unread badge and latest preview as IM list metadata", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document } = app;
    const publicRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /世界广场/.test(node.textContent || ""),
    );

    assert.ok(publicRoom);
    assert.equal(publicRoom.classList.contains("room-button-unread"), true);
    assert.equal(publicRoom.querySelector(".room-unread-badge")?.textContent, "4");
    assert.match(publicRoom.querySelector(".room-preview")?.textContent || "", /当前频道正常/);
    assert.match(publicRoom.querySelector(".room-activity")?.textContent || "", /09:44/);
  } finally {
    app.cleanup();
  }
});

test("creative resident rail searches residents and exposes avatar room entry confirmation", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document, window } = app;
    const searchInput = document.querySelector("#room-search-input");
    const residentRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /内测同伴|居民私信/.test(node.textContent || ""),
    );
    const publicRoom = Array.from(document.querySelectorAll(".room-button")).find((node) =>
      /世界广场/.test(node.textContent || ""),
    );

    assert.ok(searchInput);
    assert.match(searchInput.placeholder || "", /搜索居民/);
    assert.ok(residentRoom);
    assert.ok(publicRoom);
    publicRoom.click();

    searchInput.value = "内测";
    searchInput.dispatchEvent({ type: "input", target: searchInput });

    const filteredRooms = Array.from(document.querySelectorAll(".room-button"));
    assert.ok(filteredRooms.length >= 1);
    assert.ok(filteredRooms.every((node) => /内测同伴|居民私信/.test(node.textContent || "")));

    const avatar = filteredRooms[0]?.querySelector(".room-avatar[data-resident-room-entry]");
    let confirmMessage = "";
    window.confirm = (message) => {
      confirmMessage = message;
      return true;
    };

    assert.ok(avatar);
    avatar.click();

    assert.match(confirmMessage, /进入「.+」的房间私聊？/);
    assert.match(document.querySelector(".room-button.active")?.textContent || "", /内测同伴|居民私信/);
    assert.equal(searchInput.value, "");
  } finally {
    app.cleanup();
  }
});

test("creative rail CSS keeps IM metadata visible in the session list", serial, async () => {
  const creativeCss = await fs.readFile(new URL("../styles.creative.css", import.meta.url), "utf8");
  const pixelCss = await fs.readFile(new URL("../styles.pixel-map.css", import.meta.url), "utf8");
  const hiddenRule = creativeCss.match(/\.creative-rail \.room-section-header,[\s\S]*?\{\s*display: none !important;\s*\}/)?.[0] ?? "";
  const pixelHiddenRule = pixelCss.match(/body\[data-shell-page="hub"\]\[data-shell-variant="public-square"\] \.public-square-rail-head,[\s\S]*?\{\s*display: none;\s*\}/)?.[0] ?? "";

  assert.ok(hiddenRule, "expected creative rail hidden metadata rule");
  assert.doesNotMatch(hiddenRule, /\.creative-rail \.room-preview/);
  assert.doesNotMatch(hiddenRule, /\.creative-rail \.room-badges/);
  assert.doesNotMatch(pixelHiddenRule, /body\[data-shell-variant="creative-terminal"\] \.creative-room-list/);
  assert.match(creativeCss, /\.creative-rail-search/);
  assert.match(creativeCss, /\.creative-room-list\s*\{[\s\S]*overflow: auto;/);
  assert.match(creativeCss, /\.creative-rail \.room-avatar\[data-resident-room-entry\]/);
  assert.match(creativeCss, /\.creative-rail \.room-unread-badge/);
});

test("selecting another room keeps the composer editable and moves the active marker", serial, async () => {
  const app = await loadUserShellApp();
  try {
    const { document } = app;
    const composerInput = document.querySelector("#composer-input");
    const roomButtons = document.querySelectorAll(".room-button");
    const secondRoom = roomButtons[1];
    const initialPlaceholder = composerInput?.placeholder;
    const secondRoomId = secondRoom?.dataset?.roomId;
    const roomStageCanvas = document.querySelector("#room-stage-canvas");
    const portraitCanvas = document.querySelector("#room-stage-portrait-canvas");
    const roomStageSide = document.querySelector(".conversation-stage-side");
    const chatDetailContent = document.querySelector("#chat-detail-content");
    const chatDetailPanel = document.querySelector(".chat-detail");
    const summaryTitle = document.querySelector("#chat-detail-summary-title");
    const summaryCopy = document.querySelector("#chat-detail-summary-copy");
    const cardShell = document.querySelector("#chat-detail-card-shell");
    const cardTitle = document.querySelector("#chat-detail-card-title");
    const cardAvatar = document.querySelector("#chat-detail-card-avatar");
    const cardMeta = document.querySelector("#chat-detail-card-meta");
    const cardActions = document.querySelector("#chat-detail-card-actions");
    const composerTip = document.querySelector(".composer-tip");

    assert.ok(secondRoom);
    secondRoom.click();

    assert.equal(composerInput?.disabled, false);
    assert.equal(document.querySelector(".room-button.active")?.dataset?.roomId, secondRoomId);
    assert.notEqual(composerInput?.placeholder, initialPlaceholder);
    assert.equal(document.body.dataset.workspace, "chat");
    assert.equal(document.body.dataset.roomVariant, "city");
    assert.equal(document.body.dataset.roomMotif, "watchtower");
    assert.equal(roomStageCanvas?.dataset?.variant, "city");
    assert.equal(roomStageCanvas?.dataset?.motif, "watchtower");
    assert.equal(portraitCanvas?.dataset?.variant, "city");
    assert.equal(portraitCanvas?.dataset?.kind, "portrait");
    assert.equal(portraitCanvas?.dataset?.motif, "sentinel");
    assert.equal(portraitCanvas?.dataset?.monogram, "巡");
    assert.equal(chatDetailPanel?.dataset?.roomVariant, "city");
    assert.equal(chatDetailPanel?.dataset?.roomMotif, "watchtower");
    assert.equal(cardShell?.dataset?.roomVariant, "city");
    assert.equal(cardShell?.dataset?.roomMotif, "watchtower");
    assert.match(document.querySelector("#masthead-title")?.textContent || "", /公共频道|群聊现场/);
    assert.match(document.querySelector("#hero-note")?.textContent || "", /公共频道|公告|跨城讨论/);
    assert.match(summaryTitle?.textContent || "", /公共频道|频道状态/);
    assert.match(summaryCopy?.textContent || "", /公告|围观|跨城讨论|公共/);
    assert.match(cardTitle?.textContent || "", /公共频道|频道状态|角色卡/);
    assert.match(cardAvatar?.textContent || "", /巡|城|公/);
    assert.match(cardMeta?.textContent || "", /角色|当前|状态|公共频道/);
    assert.match(cardActions?.textContent || "", /私聊/);
    assert.match(cardActions?.textContent || "", /委托/);
    assert.match(cardActions?.textContent || "", /交易/);
    assert.match(document.querySelector("#room-stage-title")?.textContent || "", /世界广场/);
    assert.match(roomStageSide?.textContent || "", /巡逻犬|公共频道|世界广场/);
    assert.match(chatDetailContent?.textContent || "", /巡逻犬|公开频道|世界广场/);
    const whisperButton = document.querySelector('[data-card-action="私聊"]');
    assert.ok(whisperButton);
    whisperButton.click();

    assert.equal(composerInput?.value, "私聊：");
    assert.equal(whisperButton.classList.contains("is-active"), true);
    assert.match(composerTip?.textContent || "", /当前动作 私聊/);
  } finally {
    app.cleanup();
  }
});

test("user compatibility route redirects to creative.html without old UI", serial, async () => {
  const html = await fs.readFile(new URL("../user.html", import.meta.url), "utf8");

  // 不再暴露旧住宅 UI
  assert.equal(html.includes('class="panel governance"'), false);
  assert.equal(html.includes('class="panel auth"'), false);
  assert.equal(html.includes('class="identity-row"'), false);
  assert.equal(html.includes('app-user-shell'), false);
  assert.equal(html.includes('wechat-shell'), false);
  assert.equal(html.includes('id="room-stage-canvas"'), false);

  // 保留 query 参数的跳转逻辑
  assert.equal(html.includes('window.location.replace'), true);
  assert.equal(html.includes('creative.html'), true);
  assert.equal(html.includes('URLSearchParams'), true);
});

test("deprecated user styles are not part of the web runtime", serial, async () => {
  const html = await fs.readFile(new URL("../user.html", import.meta.url), "utf8");
  const entries = ["../index.html", "../creative.html", "../admin.html", "../admin-ds.html", "../unified.html", "../world-square.html"];

  assert.equal(html.includes("styles.user.css"), false);
  await assert.rejects(
    fs.readFile(new URL("../styles.user.css", import.meta.url), "utf8"),
    /ENOENT/,
  );

  for (const entry of entries) {
    const source = await fs.readFile(new URL(entry, import.meta.url), "utf8");
    assert.equal(source.includes("styles.user.css"), false, `${entry} must not load deprecated user CSS`);
  }
});
