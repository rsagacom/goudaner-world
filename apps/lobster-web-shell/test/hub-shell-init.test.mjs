import test from "node:test";
import assert from "node:assert/strict";

import { loadAdminShellApp, loadHubShellApp } from "./fake-dom.mjs";

const serial = { concurrency: false };

test("hub shell keeps the entry masthead and does not inject workspace chrome", async () => {
  const app = await loadHubShellApp();

  try {
    const { document } = app;

    assert.equal(document.querySelector("#masthead-eyebrow")?.textContent, "我和狗蛋儿的家");
    assert.equal(document.querySelector("#masthead-title")?.textContent, "选一个房间开始");
    assert.equal(document.querySelector(".workspace-switcher"), null);
    assert.match(document.querySelector("#shell-mode-badge")?.textContent || "", /入口：聊天入口/);
  } finally {
    app.cleanup();
  }
});

test("hub shell ignores generated bootstrap gateway during local preview", async () => {
  const app = await loadHubShellApp({ useGeneratedFixtures: true });

  try {
    const { document } = app;

    assert.doesNotMatch(document.querySelector("#gateway-state")?.textContent || "", /50651/);
    assert.doesNotMatch(document.querySelector("#transport-state")?.textContent || "", /网关轮询中/);
    assert.doesNotMatch(document.querySelector("#provider-state")?.textContent || "", /50651/);
  } finally {
    app.cleanup();
  }
});

test("hub shell fails closed instead of showing generated rooms when Gateway shell state fails", async () => {
  const app = await loadHubShellApp({
    useGeneratedFixtures: true,
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    gatewayShellStateShouldFail: true,
  });

  try {
    const { document } = app;
    assert.equal(document.querySelectorAll(".room-button").length, 0);
    assert.equal(document.querySelectorAll(".message-row").length, 0);
    assert.equal(document.body.dataset.gatewayConnection, "offline");
  } finally {
    app.cleanup();
  }
});

test("hub shell scopes gateway state by current identity for viewer projection", serial, async () => {
  const app = await loadHubShellApp({
    useGeneratedFixtures: true,
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });

  try {
    const { fetchCalls } = app;
    assert.ok(
      fetchCalls.some((call) => call.startsWith("http://127.0.0.1:50651/v1/shell/state?resident_id=")),
      "hub page should request shell state with resident_id",
    );
  } finally {
    app.cleanup();
  }
});

test("admin shell reloads resident-scoped projection after identity changes", serial, async () => {
  const app = await loadAdminShellApp({
    useGeneratedFixtures: true,
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
  });

  try {
    const { document, fetchCalls } = app;
    assert.ok(
      fetchCalls.some((call) => call.startsWith("http://127.0.0.1:50651/v1/shell/state?resident_id=")),
      "admin page should request shell state with resident_id",
    );

    const identityInput = document.querySelector("#identity-input");
    assert.ok(identityInput, "admin page should expose identity input");
    identityInput.value = "阿梨";
    identityInput.dispatchEvent(new Event("change", { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 20));

    assert.ok(
      fetchCalls.some((call) => call.includes("/v1/shell/state?resident_id=%E9%98%BF%E6%A2%A8")),
      "admin page should reload shell state with the updated resident_id",
    );
  } finally {
    app.cleanup();
  }
});
