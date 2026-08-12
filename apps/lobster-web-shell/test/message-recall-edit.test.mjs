import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

import { loadUserShellApp } from "./fake-dom.mjs";

const serial = { concurrency: false };
const TEST_SESSION_TOKEN = "lbst_test_session_token";

async function flushAsyncWork() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function makePayloadWithMessages(messages) {
  return {
    state_version: "shell:v1:test-message",
    conversation_shell: {
      active_conversation_id: "room:test:lobby",
      conversations: [
        {
          conversation_id: "room:test:lobby",
          title: "测试会话",
          subtitle: "测试",
          meta: "",
          kind_hint: "public",
          participant_label: null,
          route_label: null,
          list_summary: "测试列表摘要",
          status_line: "测试状态行",
          thread_headline: "测试线程",
          chat_status_summary: "聊天正常",
          queue_summary: null,
          preview_text: null,
          last_activity_label: "刚刚",
          activity_time_label: "刚刚",
          overview_summary: "测试概述",
          context_summary: "测试上下文",
          member_count: 3,
          caretaker: null,
          detail_card: null,
          workflow: null,
          inline_actions: [],
          messages,
        },
      ],
    },
    scene_render: {
      scenes: [
        {
          conversation_id: "room:test:lobby",
          scene_banner: null,
          scene_summary: "测试场景",
          room_variant: "city",
          room_motif: "plaza",
          stage: null,
          portrait: null,
        },
      ],
    },
  };
}

test("recalled message renders 消息已撤回 without leaking original text", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, emitEventSource } = app;

    const payload = makePayloadWithMessages([
      {
        sender: "qa",
        timestamp_ms: 1713240000000,
        timestamp_label: "09:00",
        text: "这条消息已被撤回",
        is_recalled: true,
        recalled_by: "qa",
        recalled_at_ms: 1713240001000,
      },
    ]);
    emitEventSource("shell-state", payload);
    await flushAsyncWork();
    await flushAsyncWork();

    const body = document.querySelector(".message-body");
    assert.ok(body);
    assert.equal(body.textContent, "消息已撤回");
    assert.equal(body.classList.contains("message-body-recalled"), true);
    assert.equal(body.textContent.includes("这条消息已被撤回"), false);

    const edited = document.querySelector(".message-edited");
    assert.equal(edited, null);
  } finally {
    app.cleanup();
  }
});

test("edited message renders updated text and 已编辑 chip", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, emitEventSource } = app;

    const payload = makePayloadWithMessages([
      {
        sender: "qa",
        timestamp_ms: 1713240000000,
        timestamp_label: "09:00",
        text: "更新后的内容",
        is_edited: true,
        edited_by: "qa",
        edited_at_ms: 1713240001000,
      },
    ]);
    emitEventSource("shell-state", payload);
    await flushAsyncWork();
    await flushAsyncWork();

    const body = document.querySelector(".message-body");
    assert.ok(body);
    assert.equal(body.textContent, "更新后的内容");
    assert.equal(body.classList.contains("message-body-recalled"), false);

    const edited = document.querySelector(".message-edited");
    assert.ok(edited);
    assert.equal(edited.textContent, "已编辑");
  } finally {
    app.cleanup();
  }
});

test("recall takes precedence over edit mark", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, emitEventSource } = app;

    const payload = makePayloadWithMessages([
      {
        sender: "qa",
        timestamp_ms: 1713240000000,
        timestamp_label: "09:00",
        text: "既编辑又撤回的消息",
        is_recalled: true,
        recalled_by: "qa",
        recalled_at_ms: 1713240002000,
        is_edited: true,
        edited_by: "qa",
        edited_at_ms: 1713240001000,
      },
    ]);
    emitEventSource("shell-state", payload);
    await flushAsyncWork();
    await flushAsyncWork();

    const body = document.querySelector(".message-body");
    assert.ok(body);
    assert.equal(body.textContent, "消息已撤回");
    assert.equal(body.classList.contains("message-body-recalled"), true);

    const edited = document.querySelector(".message-edited");
    assert.equal(edited, null);
  } finally {
    app.cleanup();
  }
});

test("message body uses textContent not innerHTML for recalled content", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, emitEventSource } = app;

    const payload = makePayloadWithMessages([
      {
        sender: "qa",
        timestamp_ms: 1713240000000,
        timestamp_label: "09:00",
        text: "<script>alert('xss')</script>",
        is_recalled: true,
        recalled_by: "qa",
        recalled_at_ms: 1713240001000,
      },
    ]);
    emitEventSource("shell-state", payload);
    await flushAsyncWork();
    await flushAsyncWork();

    const body = document.querySelector(".message-body");
    assert.ok(body);
    assert.equal(body.textContent, "消息已撤回");
    assert.equal(body.innerHTML, "消息已撤回");
  } finally {
    app.cleanup();
  }
});

test("message body uses textContent not innerHTML for edited content", serial, async () => {
  const app = await loadUserShellApp({
    useGeneratedFixtures: true,
    generatedShellFixture: "generated/state.contract.json",
    locationSearch: "?gateway=http://127.0.0.1:50651",
    gatewayBaseUrl: "http://127.0.0.1:50651",
    localStorageEntries: {
      "lobster-identity": "qa",
      "lobster-session-token": TEST_SESSION_TOKEN,
    },
  });
  try {
    const { document, emitEventSource } = app;

    const payload = makePayloadWithMessages([
      {
        sender: "qa",
        timestamp_ms: 1713240000000,
        timestamp_label: "09:00",
        text: "<b>bold</b>",
        is_edited: true,
        edited_by: "qa",
        edited_at_ms: 1713240001000,
      },
    ]);
    emitEventSource("shell-state", payload);
    await flushAsyncWork();
    await flushAsyncWork();

    const body = document.querySelector(".message-body");
    assert.ok(body);
    assert.equal(body.textContent, "<b>bold</b>");
    // Fake DOM 中 textContent setter 会同步 _innerHTML = _textContent；
    // 若用 innerHTML setter，则 _textContent 会被 stripTags 清空标签。
    // 两者相等即可证明走了 textContent 路径，未使用 innerHTML。
    assert.equal(body.innerHTML, body.textContent);
  } finally {
    app.cleanup();
  }
});
