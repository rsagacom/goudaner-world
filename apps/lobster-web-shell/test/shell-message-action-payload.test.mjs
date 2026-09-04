import { test } from "node:test";
import assert from "node:assert/strict";
import {
  gatewayMessagePayloadForState,
  editMessagePayloadForState,
  recallMessagePayloadForState,
} from "../shell-message-action-payload.js";

const deps = { currentIdentity: () => "resident-a", languageTag: "zh-CN" };

test("gatewayMessagePayload: 完整字段", () => {
  const payload = gatewayMessagePayloadForState("room-1", "hello", "quick", deps);
  assert.deepEqual(payload, {
    room_id: "room-1",
    sender: "resident-a",
    text: "hello",
    attachment_id: undefined,
    quick_action: "quick",
    device_id: "browser-shell",
    language_tag: "zh-CN",
  });
});

test("gatewayMessagePayload: 空 quickAction 时为 undefined", () => {
  const payload = gatewayMessagePayloadForState("room-1", "hi", "", deps);
  assert.equal(payload.quick_action, undefined);
});

test("gatewayMessagePayload: 无 languageTag 回退 zh-CN", () => {
  const payload = gatewayMessagePayloadForState("r", "x", "", { currentIdentity: () => "u" });
  assert.equal(payload.language_tag, "zh-CN");
});

test("editMessagePayload: text 被 trim", () => {
  const payload = editMessagePayloadForState("r1", "m1", "  hi  ", deps);
  assert.deepEqual(payload, { room_id: "r1", message_id: "m1", actor: "resident-a", text: "hi" });
});

test("recallMessagePayload: 字段", () => {
  const payload = recallMessagePayloadForState("r1", "m1", deps);
  assert.deepEqual(payload, { room_id: "r1", message_id: "m1", actor: "resident-a" });
});
