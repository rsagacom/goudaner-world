/*
   shell-private-room-locked-card.test.mjs — 未授权私宅 stage 空态卡片纯模型与接线测试
*/
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";

import { privateRoomLockedCardModel } from "../shell-private-room-locked-card.js";

async function readShellModule(name) {
  return fs.readFile(new URL(`../${name}`, import.meta.url), "utf8");
}

const lockedLogin = {
  className: "resident-room-access-note is-locked",
  text: "登录后才能访问 tym331 的私宅。",
  isError: true,
};
const lockedFriend = {
  className: "resident-room-access-note is-locked",
  text: "访问 tym331 的私宅需要先成为好友，请点「申请好友」。",
  isError: true,
};
const pendingOutgoing = {
  className: "resident-room-access-note is-pending",
  text: "已向 tym331 申请好友；对方接受后才能进入私宅。",
  isError: false,
};
const actionableIncoming = {
  className: "resident-room-access-note is-actionable",
  text: "tym331 已发来好友申请；先点「接受好友」再进入私宅。",
  isError: false,
};

test("locked card model reuses timeline-empty-card structure with tone class", () => {
  const card = privateRoomLockedCardModel(lockedFriend, { displayName: "tym331" });
  assert.match(card.className, /timeline-empty-card/);
  assert.match(card.className, /private-room-locked-card is-locked/);
  assert.equal(card.titleClassName, "timeline-empty-title");
  assert.equal(card.copyClassName, "timeline-empty-copy");
  assert.equal(card.actionClassName, "timeline-empty-action");
});

test("titles and actions follow the five access states", () => {
  assert.equal(privateRoomLockedCardModel(lockedLogin, {}).titleText, "这道门暂时进不去");
  assert.match(privateRoomLockedCardModel(lockedLogin, {}).actionText, /先登录/);
  assert.match(
    privateRoomLockedCardModel(lockedFriend, { displayName: "tym331" }).actionText,
    /申请好友.*tym331/,
  );
  assert.equal(privateRoomLockedCardModel(pendingOutgoing, {}).titleText, "还差一步");
  assert.match(privateRoomLockedCardModel(pendingOutgoing, {}).actionText, /无需重复申请/);
  assert.equal(privateRoomLockedCardModel(actionableIncoming, {}).titleText, "对方在等你回应");
  assert.match(privateRoomLockedCardModel(actionableIncoming, {}).actionText, /接受好友/);
});

test("copy text always carries the gateway-derived prompt, never a fabricated one", () => {
  for (const prompt of [lockedLogin, lockedFriend, pendingOutgoing, actionableIncoming]) {
    assert.equal(privateRoomLockedCardModel(prompt, {}).copyText, prompt.text);
  }
  assert.equal(privateRoomLockedCardModel(null, {}), null);
});

test("app.js mounts the locked card from the access-prompt branch", async () => {
  const source = await readShellModule("app.js");
  assert.match(source, /import \{ privateRoomLockedCardModel \} from "\.\/shell-private-room-locked-card\.js";/);
  assert.match(source, /function renderPrivateRoomLockedCard\(accessPrompt, displayName\)/);
  assert.match(
    source,
    /setGovernanceStatus\(accessPrompt\.text, accessPrompt\.isError, accessPrompt\.className\);\s*\n\s*renderPrivateRoomLockedCard\(accessPrompt, displayName\);/,
  );
  // 卡片经 createTimelineEmptyStateNode 挂载，且先清空 timeline
  assert.match(source, /clearChildren\(timelineEl\);\s*\n\s*timelineEl\.appendChild\(createTimelineEmptyStateNode\(cardModel\)\);/);
});

test("locked card styles stay dark-on-dark inside the user shell", async () => {
  const css = await readShellModule("styles.user-shell.css");
  assert.match(css, /\.private-room-locked-card \{[\s\S]*?background: rgba\(22, 16, 12, 0\.88\)/);
  assert.match(css, /\.private-room-locked-card \{[\s\S]*?border: 1px solid #3a2f28/);
  assert.match(css, /\.private-room-locked-card\.is-locked \.timeline-empty-action/);
  assert.match(css, /\.private-room-locked-card\.is-actionable|\.private-room-locked-card \.timeline-empty-action/);
});
