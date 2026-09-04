import test from "node:test";
import assert from "node:assert/strict";

let sendModule = null;
try {
  sendModule = await import("../shell-message-send.js");
} catch {
  sendModule = null;
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

function createHarness({
  roomId = "room-1",
  gatewayConnected = true,
  loginRequired = false,
  postResult = null,
  refreshResult = null,
} = {}) {
  const calls = [];
  let controller;
  controller = sendModule?.createMessageSendController?.({
    getContext: () => ({ roomId, gatewayConnected, loginRequired }),
    commitLocal: (request) => {
      calls.push(["local", request, controller.isSending()]);
      return true;
    },
    buildPayload: (request) => {
      calls.push(["payload", request]);
      return { room_id: request.roomId, text: request.text };
    },
    prepareGateway: (request) => {
      calls.push(["prepare", request, controller.isSending()]);
      return "pending-1";
    },
    postGateway: async (request) => {
      calls.push(["post", request, controller.isSending()]);
      if (postResult) await postResult;
    },
    clearSendError: (request) => calls.push(["clear-error", request]),
    refreshGateway: async (request) => {
      calls.push(["refresh", request, controller.isSending()]);
      if (refreshResult) await refreshResult;
    },
    clearPending: (request) => calls.push(["clear-pending", request]),
    handleFailure: (request) => {
      calls.push(["failure", request, controller.isSending()]);
      return new Error(request.posted ? "posted but sync failed" : "send failed");
    },
    onSettled: () => calls.push(["settled", controller.isSending()]),
  });
  return { calls, controller };
}

test("message send controller exposes one instance-owned in-flight boundary", () => {
  assert.equal(typeof sendModule?.createMessageSendController, "function");
  const { controller } = createHarness();
  assert.equal(controller.isSending(), false);
  assert.equal(typeof controller.send, "function");
});

test("message send controller keeps local preview sends outside the gateway lifecycle", async () => {
  const { calls, controller } = createHarness({ gatewayConnected: false });

  const result = await controller.send("本地消息", { quickAction: "续聊" });

  assert.equal(result, true);
  assert.deepEqual(calls, [[
    "local",
    { roomId: "room-1", text: "本地消息", quickAction: "续聊", attachmentId: "" },
    false,
  ]]);
  assert.equal(controller.isSending(), false);
});

test("message send controller commits gateway send only after post and refresh", async () => {
  const { calls, controller } = createHarness();

  const result = await controller.send("网关消息", { quickAction: "整理" });

  assert.equal(result, true);
  assert.deepEqual(calls.map(([name]) => name), [
    "payload",
    "prepare",
    "post",
    "clear-error",
    "refresh",
    "clear-pending",
    "settled",
  ]);
  assert.equal(calls.find(([name]) => name === "prepare")[2], true);
  assert.equal(calls.find(([name]) => name === "refresh")[2], true);
  assert.equal(calls.at(-1)[1], false);
});

test("message send controller distinguishes post failure from post-refresh failure", async () => {
  const postFailure = Promise.reject(new Error("post unavailable"));
  postFailure.catch(() => {});
  const postHarness = createHarness({ postResult: postFailure });
  await assert.rejects(postHarness.controller.send("失败消息"), /send failed/);
  assert.equal(postHarness.calls.find(([name]) => name === "failure")[1].posted, false);
  assert.equal(postHarness.controller.isSending(), false);

  const refreshFailure = Promise.reject(new Error("refresh unavailable"));
  refreshFailure.catch(() => {});
  const refreshHarness = createHarness({ refreshResult: refreshFailure });
  await assert.rejects(refreshHarness.controller.send("可能已发出"), /posted but sync failed/);
  assert.equal(refreshHarness.calls.find(([name]) => name === "failure")[1].posted, true);
  assert.equal(refreshHarness.controller.isSending(), false);
});

test("message send controller rejects login-required sends before mutating UI state", async () => {
  const { calls, controller } = createHarness({ loginRequired: true });

  await assert.rejects(controller.send("未登录消息"), /请先登录后发送/);

  assert.deepEqual(calls, []);
  assert.equal(controller.isSending(), false);
});

test("message send controller suppresses concurrent duplicate calls at the source", async () => {
  const post = deferred();
  const { calls, controller } = createHarness({ postResult: post.promise });
  const first = controller.send("第一条");
  assert.equal(controller.isSending(), true);

  const second = await controller.send("重复消息");
  assert.equal(second, false);
  assert.equal(calls.filter(([name]) => name === "post").length, 1);

  post.resolve();
  assert.equal(await first, true);
  assert.equal(controller.isSending(), false);
});
