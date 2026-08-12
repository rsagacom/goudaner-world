# 我和狗蛋儿的家 · Goudaner World

「我和狗蛋儿的家」（Goudaner World）是一套以即时通讯（IM）为核心的城邦式交流系统。它先解决一个城邦里最真实的事：居民能够登录、找到人、进入房间、私聊、群聊并参与共建；再把世界入口、城邦治理和跨城互联作为清晰的外层能力逐步展开。

当前 H5 是主交互入口：主城承载公共群聊，住宅承载私聊，世界广场提供只读公共发现，城主在同一套会话语境里处理公告、提醒和维护。Rust Gateway 提供统一状态与权限合同，TUI/CLI 消费同一套合同，保证浏览器、终端与本地智能体共享同一条 IM 主线，同时遵守各自的权限边界。

## 内测入口说明（中文）

入口口径统一为“先聊天，再展开共建入口”：H5 先承接主流程，TUI 作为消费同一套 gateway 合同的并行终端客户端，城主与居民共用同一套世界观，但在同窗里拥有不同权限，不把页面说成旧式工具页或工具墙。

- 启动页：`apps/lobster-web-shell/index.html`
- 居民投影：`apps/lobster-web-shell/creative.html`
- 城主投影：`apps/lobster-web-shell/admin.html`
- 城主后台新视觉基线：`apps/lobster-web-shell/admin-ds.html`
- 世界入口：`apps/lobster-web-shell/unified.html`
- 世界广场：`apps/lobster-web-shell/world-square.html`
- 默认网关示例：`http://127.0.0.1:8787`
- 快速验收说明：[`docs/WEB_SHELL_ACCEPTANCE.md`](docs/WEB_SHELL_ACCEPTANCE.md)
- 线性后台功能目录：[`docs/ADMIN_FUNCTION_CATALOG.md`](docs/ADMIN_FUNCTION_CATALOG.md)
- 前端技术债治理计划：[`docs/FRONTEND_DEBT_REDUCTION_PLAN.md`](docs/FRONTEND_DEBT_REDUCTION_PLAN.md)

## 本地预览

最小直接预览方式是将 `apps/lobster-web-shell` 作为静态目录挂起来。示例入口地址如下（前缀 `http://127.0.0.1:8080` 取决于你的静态服务器）：

1. `http://127.0.0.1:8080/` —— 主城群聊入口。
2. `http://127.0.0.1:8080/creative.html` —— 正式居民住宅 / 私聊 IM，接真实 gateway 时使用居民登录态过滤会话。
3. `http://127.0.0.1:8080/user.html` —— 旧链接兼容入口，保留 query 参数后跳转到 `creative.html`。
4. `http://127.0.0.1:8080/admin.html` —— 城主治理兜底页，当前只保基础可用和移动端兜底。
5. `http://127.0.0.1:8080/admin-ds.html` —— DeepSeek V4 Pro 后台视觉方案，已作为后续正式后台样式基线保留。
6. `http://127.0.0.1:8080/unified.html` —— 世界入口，地铁候车站式导航页，不展示聊天框。
7. `http://127.0.0.1:8080/world-square.html` —— 世界广场，只读公共入口，展示公告、公共讨论和跨城发现摘要。

运行方式：

```bash
cd apps/lobster-web-shell
python3 -m http.server 8080
```

需要接真实消息时，先起一个本地 gateway（默认 `http://127.0.0.1:8787`），然后把 `?gateway=http://127.0.0.1:8787` 拼到上面的任意页面 URL 上。

详细的快速验收请直接看 [`docs/WEB_SHELL_ACCEPTANCE.md`](docs/WEB_SHELL_ACCEPTANCE.md)。

脚本化 H5 smoke：

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat
./scripts/smoke-web-shell.sh
```

这条 smoke 当前覆盖：

- `index.html` / `creative.html` / `admin.html` / `admin-ds.html` / `unified.html` / `world-square.html` 的静态入口结构
- `creative.html` 的 fake-DOM 启动与默认聊天主路径
- `user.html` 的兼容跳转：保留 query 参数后重定向到 `creative.html`
- 切换会话后输入区 placeholder 跟随当前线程刷新

真实 gateway shell 双端 HTTP smoke：

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat
./scripts/smoke-shell-dual-http.sh
./scripts/smoke-shell-direct-http.sh
```

这些 smoke 会启动临时 gateway：公共房间用 `qa-a` 发送 shell 消息并验证 `qa-b` 的 `/v1/shell/events` 与 `/v1/shell/state` 都能看到同一条消息；私聊用 `qa-a`/`qa-b` 打开 `dm:qa-a:qa-b`，验证 send/edit/recall 都会进入 peer SSE 与 state projection，并确认 `qa-c` 不可见且不能写入。

额外的外部入口 smoke：

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat
BASE_URL=https://<node>.<tailnet>.ts.net ./scripts/smoke-public-ingress.sh
```

它会验证外部入口的：

- `/`
- `GET /health`
- `HEAD /health`
- `/v1/provider`

当前页面边界：

- `creative.html` 目前固定为住宅 / 私聊入口，默认直接进入房间列表与消息流
- `creative.html?gateway=...` 会在访客态显示紧凑邮箱验证码登录片，登录成功后保存 `resident_id` 并重新拉取该居民可见会话
- 真实 gateway 模式下，`creative.html` 会请求 `GET /v1/shell/state?resident_id=<当前身份>`；未登录身份为 `访客`，只能看到公共/访客可见入口，且不能向私聊或房间发送正式消息
- H5 实时读侧合同优先使用 `GET /v1/shell/events?resident_id=<当前身份>` 的 SSE `shell-state` 事件，`ShellState` 带 `state_version` 供客户端判断快照变化；需要等待变化时可带 `after=<state_version>&wait_ms=<ms>`，网关会等待到版本变化或超时，超时无变化时返回同版本快照和 `shell-heartbeat`，客户端按最新 `state_version` 重连；写侧仍走 `POST /v1/shell/message`，成功回执必须带 `delivery_status="delivered"`、`message_id`、`delivered_at_ms`、`sender` 和规范化 `text`，可选 `reply_to_message_id` 用于微信式回复引用；后续状态投影里的同条消息也会带 `delivery_status="delivered"`；撤回走 `POST /v1/shell/message/recall`，编辑走 `POST /v1/shell/message/edit`，H5 和 CLI/TUI 读侧投影都保留审计记录并返回 `is_recalled/recalled_by/recalled_at_ms` 与 `is_edited/edited_by/edited_at_ms`
- H5 与 CLI 写入统一使用 gateway 文本合同：正文先 trim，trim 后必须为 1-2000 字符；空白消息和超长消息直接由 gateway 返回失败
- `user.html` 只保留为旧链接兼容入口，不再展示独立住宅 UI；它会保留 query 参数并跳转到 `creative.html`
- `auth / world / governance` 这些附加 workspace 现在只在非 `user` 页面暴露

### 居民主视角（Web 主入口）

1. 打开用户端页面，阅读“如何开始”指引，优先进入会话列表。
2. 住宅页当前默认直接进入聊天主路径；接入真实 gateway 后，访客态先用邮箱验证码登录，登录成功后刷新为该居民视角。
3. 用户端默认是“两栏 IM”：左侧收件箱，右侧线程和输入框。会话摘要优先显示最后一条消息、时间、未读和草稿。
4. 底部输入区明确按“聊天输入区”来设计：先提示当前线程和发送状态，再露出真正的输入框和发送按钮。
5. 终端默认先把焦点放在会话导航；按 `i` 进入输入，`Enter` 发送，`Shift+Enter` 插入换行，`ArrowUp` 快速恢复上一条。
6. `/open`、`/world` 等命令仍然可用，但只算补充入口。`/governance` 仅在当前 surface 已挂出治理会话时才会切换；主流程仍然是聊天、提醒和共建留言。

### 城主与共建视角

1. 城主投影优先用于处理待跟进聚落、公告、安全提醒和互联巡检结果。
2. 城主投影是“会话优先的线性共建台”：左侧居民与聚落列表，中间当前线程，右侧是少量、明确、可解释的共建动作。
3. 底部输入区仍然像聊天框，但默认语义是跟进、公告和处置，而不是工具操作。
4. 右侧区域只在分析、调度或公告时展开，不要让它抢走线程本身的主位。
5. 任何巡检动作都应生成可见卡片，带上当前责任人、影响范围和处理结果。

### 聊天交互提示（Web 主入口）

- 默认先在导航区，按 **i** 进入输入；进入输入后，**Enter** 发送，**Shift+Enter** 插入换行，**ArrowUp** 复用上一条。
- 透明助手口令：`/assistant`, `/owner`, `/dog`，直接叫 OpenClaw 共建助手辅助留言、巡检、公告。
- 保持会话上下文可见：输入区下方会提示同步状态、共建记忆和访客提醒；网关慢时会先显示本地待同步消息。


## 📊 当前开发进度 (2026-08-13)

| 模块 | 测试数 | Clippy | 状态 |
|------|--------|--------|------|
| Gateway (Rust) | 323 | ✅ | 私宅访问确权+好友关系流, 设备管理, 场景权限, URL 安全合同, 审核持久化, 38+ HTTP 端点 |
| TUI (Rust/ratatui) | 233 | ✅ | 用户端/城主端/世界广场/私聊终端 |
| CLI (Rust) | 50 | ✅ | login/nickname/directory/snapshot/moderate/admin 命令 |
| H5 Web Shell (JS) | 1419 | — | 7 页面, 场景交互, admin-ds 写操作护栏, Gateway 真源投影, Gateway fail-closed/会话失效/合法空投影在线, app.js 7,529 行 |
| crypto-mls | 20 | ✅ | AES-256-GCM + HKDF 前向安全 |
| ai-sidecar | 7 | ✅ | HTTP AI 助手 + 流式 |
| chat-core | 20 | ✅ | 合同定义, SceneImageLayer day/night |
| chat-storage | 17 | ✅ | 文件存储, 原子写 |
| **总计** | **~2,063** | **零警告** | 全模块通过；完整 release gate 已包含 provider federation |

最近更新（8-13）：
- 🛡️ admin-ds Gateway 会话失效闭环：后台 GET/POST 收到 401/403 统一清除过期 Bearer、降为访客并恢复登录 HUD；真实双浏览器已覆盖 401→重新登录入口，Web 1419、layout、realness 与完整 release gate 全绿
- 🔑 H5 Gateway 会话失效闭环：shell state 401/403 清除过期 Bearer、保留重新登录提示并阻断 stale-session 轮询/SSE；Web 1418、layout、realness 全绿
- 🟢 H5 合法空 Gateway 投影保持在线：新居民/无可见会话时显示真实空态但不误报 offline；不完整载荷继续 fail-closed；Web 1416、Gateway 323、完整 release gate 全绿
- 🛡️ H5 Gateway shell state fail-closed：Gateway 失败不再展示 sample/cache，provider 可达不再掩盖 IM shell offline；Web 1413、Gateway 323、完整 release gate 全绿
- 🔐 provider/mirror URL 安全合同已收口：远端 HTTPS、开发 loopback HTTP、非法持久化配置 fail-closed；native Waku/标准 MLS 仍未引入
- 🎨 H5 UI refresh P0/P1/P2 已完成并通过 Web 1404、layout、realness 与完整 release gate；生产三形态视觉复核已记录
- 🧩 H5 世界/治理/居民/房间/会话摘要/线程状态表面职责下沉；Gateway 323、TUI 233 与完整 release gate 全绿
- 🚀 Atlas 生产记录显示 `deployment.lobster-chat-production` active/verified；真实健康检查、13/13 双端 IM 验收和 UI refresh 复核均已记录
- 🏠 私宅主客访问确权：registered_all/friends_only 策略 + 房主确权端点 + 防消息泄漏
- 🤝 好友关系流：request/accept 两步流 + 居民目录按 viewer 投影 relationship_state
- 🔐 注册登录共享接线：shell-auth-standalone.js 统一 world-square/admin-ds OTP 流程
- 🛡️ admin-ds 写操作护栏：9 处 .then 假成功态修复 + 设备/场景/邀请码真实端点闭环
- 📉 app.js 当前约 7,524 行，DOM surface 持续拆分；Gateway 仍是聊天和场景状态唯一真源
- 🎨 美术护栏：day 资产去蜡黄 + 热点层透明化 + CSS 拆分防回归

完整审计: [docs/TECH_AUDIT_REPORT_20260604.md](docs/TECH_AUDIT_REPORT_20260604.md)
开发蓝图: [docs/DEVELOPMENT_BLUEPRINT.md](docs/DEVELOPMENT_BLUEPRINT.md)
工作队列: [docs/ACTIVE_WORK_QUEUE.md](docs/ACTIVE_WORK_QUEUE.md)

## Product direction

lobster-chat is intentionally built around three mutually reinforcing experiences:

1. **居民 IM** – a chat-first surface with conversation lists, timelines, and handoff-friendly inputs that look and feel like a modern messaging client.
2. **轻巡与互联守望** – passive monitoring agents that keep an eye on safety signals, surface action cards, and quietly report to the城主 when something needs attention.
3. **Personal OpenClaw 共建助手** – a trusted helper that can leave visitor notes, moderate 聚落, carry a lightweight memory, and alert the城主 or tools when needed.

These pillars sit on top of the existing expansion strategy:

- **downward:** keep the core runnable on embedded/low-resource hosts without dragging in bulky AI/GUI stacks.
- **sideways:** provide mobile H5 projections so residents can dip in via browsers before native apps ship.
- **upward:** ensure wearables and glance surfaces can consume the same core via compact interactions.

The build order remains:

1. local-first core
2. host adapter boundary
3. lightweight transport and security layers
4. wearable expansions after the base stabilizes

## Goals

- gateway contract as the only source of truth for chat and scene state
- H5 主交互入口 for current IM and SFC-style scene work
- TUI parallel mapped client for terminal-first operation and parity checks
- Linux / macOS / Windows compatibility
- lightweight core that can run headless
- Waku transport for decentralized delivery
- MLS-based security layer
- local archive with active-window sync
- optional AI sidecar for translation, summarization, and semantic assistance
- host adapter layer for embedding inside Lobster
- embedded-friendly and wearable-friendly surface design

## Workspace layout

- `crates/chat-core`: shared message, identity, room, archive, and device models
- `crates/chat-storage`: local persistence traits and in-memory timeline prototype
- `crates/host-adapter`: embedding boundary for Lobster, desktop, mobile web, and wearable hosts
- `crates/transport-waku`: Waku transport boundary and future light-client implementation
- `crates/crypto-mls`: MLS session and room security boundary
- `crates/ai-sidecar`: optional AI assist boundary
- `apps/lobster-tui`: standalone terminal client that consumes the shared gateway contract
- `apps/lobster-web-shell`: current primary H5 client for browsers and future PWA packaging
- `apps/lobster-waku-gateway`: localhost JSON gateway skeleton for sharing one Waku-facing transport service across entries
- `scripts/`: preflight, packaging, smoke verification, and server install helpers
- `docs/`: architecture and phased implementation notes
- `docs/CARETAKER_PLAYBOOK.md`: product vision for the IM-first caretaker, patrol, and visitor flows
- `docs/NOVEL_TTS_PLAYBOOK.md`: audiobook/TTS repair workflow and delivery checklist for caretaker-style agent operations

## World model

The protocol uses a `世界 / 城邦 / 居民 / 城主` model:

- `世界`: shared protocol, identity, and互联规则
- `城邦`: a hosted service domain that provides relay, storage, discovery, and public共建
- `聚落`: a conversational or community unit inside a城邦
- `居民`: a portable user identity that is not trapped inside any one城邦
- `城主`: a city-scoped authority with WeChat-group-owner-level stewardship for public spaces plus infrastructure responsibility, but without private-message plaintext access or global ownership over any resident

The public城邦目录 should be mirrored across multiple互联城邦.

## OpenClaw integration

`lobster-chat` is being designed so optional OpenClaw-powered bots can help
居民在聚落、城邦和世界相关流程中协作，而不会把
whole product into a single AI entry.

See:

- `docs/OPENCLAW_AGENT_INTERFACE.md`
The first 城邦 can act as a seed city early on, but the architecture should not depend on a permanent world server.

Key references:

- [docs/PRODUCT_CHARTER.md](docs/PRODUCT_CHARTER.md)
- [docs/WORLD_CITY_MODEL.md](docs/WORLD_CITY_MODEL.md)
- [docs/WORLD_CITY_API.md](docs/WORLD_CITY_API.md)
- [docs/WORLD_SQUARE_AND_SAFETY.md](docs/WORLD_SQUARE_AND_SAFETY.md)
- [docs/WAKU_GATEWAY_PROTOCOL.md](docs/WAKU_GATEWAY_PROTOCOL.md)
- [docs/MLS_SKELETON.md](docs/MLS_SKELETON.md)
- [docs/AUTH_AND_REGISTRATION.md](docs/AUTH_AND_REGISTRATION.md)
- [docs/RESOURCE_BUDGET.md](docs/RESOURCE_BUDGET.md)
- [docs/DEPLOYMENT_PITFALLS_AND_HARDENING.md](docs/DEPLOYMENT_PITFALLS_AND_HARDENING.md)
- [docs/TUI_RENDERING_STRATEGY.md](docs/TUI_RENDERING_STRATEGY.md)
- [docs/THEME_AND_NOMENCLATURE.md](docs/THEME_AND_NOMENCLATURE.md)
- [docs/SPATIAL_SCENE_MODEL.md](docs/SPATIAL_SCENE_MODEL.md)

## Phase 2 status

Current state already includes:

- compile-ready Rust workspace
- local-first domain model
- file-backed timeline and archive policy prototype
- host capability split for embedded / desktop / mobile web / wearable
- postcard-based compact message framing for Waku payload transport
- gateway-backed Waku adapter boundary with an in-memory light-node gateway for topic subscribe / poll / history recovery
- localhost Waku gateway process skeleton with JSON request / response contract
- gateway-side file-backed state so room timelines survive localhost gateway restarts
- localhost gateway endpoints for world, city, membership, and public-room operations
- optional upstream gateway interlinking for multi-city transport experiments
- downstream world discovery can now merge upstream city catalogs
- direction agreed: the world directory should be mirrorable across multiple cities so discovery survives loss of any single seed city
- direction agreed: the world should include a mirrored World Square plus a world-level safety coordination layer for emergency quarantine of malicious cities
- severe confirmed abuse can revoke cross-city portability for the offending resident identity, and the gateway now has resident-sanction plus hashed registration blacklist feeds for email/mobile handles
- the gateway now has a low-cost auth skeleton for email OTP registration/login plus blacklist-aware handle preflight
- the H5 projection can now request and verify email OTP login against that auth skeleton
- H5 residents can now file public-abuse reports into the world safety queue
- MLS lifecycle skeleton for direct sessions, room sessions, epoch rotation, and transport envelopes
- direct 1v1 session bootstrap API with persisted MLS skeleton state
- standalone TUI bootstrap that simulates a local message flow
- static H5 projection that consumes Rust-exported bootstrap and room state
- H5 projection gateway mode with live poll/send support and locally remembered resident handle
- H5 projection world view with city discovery, join, found-city, and public-room actions
- H5 resident directory with direct-message bootstrap against the localhost gateway
- H5 resident-visible history export for private, group, and city-public conversations
- H5 projection now exposes unread state, active-room highlighting, send / sync / empty feedback, and narrow-screen chat switching
- H5 城主互联策略 modes for `Open / Selective / Isolated`
- provider status endpoint so clients can see whether they are on local memory transport or a remote provider bridge
- provider connect / disconnect flow so a city gateway can persistently bridge to another provider without CLI-only setup
- terminal render capability model for SFC-inspired high-color terminals with graceful FC-style fallback
- city-lord moderation API for join approval, steward assignment, and public-room freeze handling
- world-square notice model plus world-safety trust/advisory skeleton
- world-directory snapshot and mirror list endpoints so clients can render a mirrored entry catalog
- world snapshot bundle endpoint with checksum metadata for mirror nodes and cached clients
- H5 world layer that now surfaces directory entries, mirrored sources, notices, and safety advisories
- release-side helper scripts for preflight, packaging, and server installation
- unified local release gate now covers CLI, auth, resident, public/direct shell, web shell, and terminal smoke in one entrypoint

## 当前 MVP 边界

当前版本不按“把全部 Phase 一次做完”推进，而是先收一版可用主线。

当前只把下面这些算作 `MVP` 范围：

- gateway 正式合同作为唯一真源：`conversation_shell / scene_render`
- `H5` 主交互入口，承接当前 IM 与 SFC 场景主路径
- `TUI` 并行映射客户端，用于终端操作、验收和后续多入口一致性
- 单城邦范围内的私聊、群聊、房间聊天主路径
- `lobster-cli` 命令行聊天通道，供 `OpenClaw / Codex / Claude Code` 等本地智能体直接发消息、看收件箱、追消息流
- 为后续真实 transport 和真实加密保留明确边界

当前明确后置：

- `World Square / World Safety` 的完整产品面
- 多城邦互联产品化
- 眼镜端实现
- 装扮编辑器、素材系统与自由摆放
- `H5` 的离线/PWA 完整体验与二阶段产品化抛光

所以现在如果你看到文档里还有更大的世界层、互联层、穿戴层设想，那些都应理解成“后续预留”，不是当前版本必须同时落地的内容。当前版本先保证 H5 和 TUI 都消费同一套正式合同，而不是让任一前端继续长私有状态。

## Localhost gateway quick start

Run the local gateway:

```bash
cargo run -p lobster-waku-gateway -- --host 127.0.0.1 --port 8787
```

Optional: choose a persistent state directory explicitly.

```bash
cargo run -p lobster-waku-gateway -- --host 127.0.0.1 --port 8787 --state-dir ./.lobster-chat-dev/gateway
```

Then point the TUI at it:

```bash
LOBSTER_WAKU_GATEWAY_URL=http://127.0.0.1:8787 \
LOBSTER_SESSION_TOKEN=<session-token> \
cargo run -p lobster-tui
```

在关闭 `LOBSTER_DEV_AUTH_BYPASS` 的真实 Gateway 上，TUI 的 `/edit`、`/recall`、`/dm`、管理读写和治理查询会透传 `Authorization: Bearer ...`。优先设置 `LOBSTER_SESSION_TOKEN`；sidecar 场景也可使用 `LOBSTER_AGENT_TOKEN`。未设置 token 仅适用于本地 dev bypass 或不需要鉴权的只读路径。

### 命令行聊天通道（`lobster-cli`）

`lobster-cli` 是挂在本地 `gateway` 旁边的一条命令行聊天通道。它不是新的独立聊天系统，也不是绕过 `gateway` 直写本地存储，而是给这些调用方补一条非图形入口：

- `OpenClaw`
- `Codex`
- `Claude Code`
- 以后需要批处理或自动化发消息的脚本

第一版只做 lobster 内部送达，不接飞书、微信、短信等外部网络。

地址格式：

- `user:<id>`：用户身份，例如 `user:zhangsan`
- `agent:<id>`：智能体身份，例如 `agent:openclaw`
- `room:<scope>:<city>:<name>`：房间身份，例如 `room:city:core-harbor:lobby`

注意：

- `send --to` 这里传的是 CLI 地址，不是内部 `conversation_id`
- 不要把 `dm:openclaw:zhangsan` 这类会话 ID 直接传给 `--to`
- 点对点消息要传 `user:...` 或 `agent:...`，再由 gateway 归一化成内部 `dm:...`

输出约定：

- 默认输出人类可读文本，适合手工排查和临时操作
- 带 `--json` 时输出结构化 JSON，适合 `OpenClaw / Codex / Claude Code` 这类本地智能体或脚本直接解析
- `tail` 人类可读输出会把已撤回消息标成 `[已撤回]` 且只显示 `消息已撤回`，把已编辑消息标成 `[已编辑]`；脚本侧应优先读取 JSON 里的 `is_recalled/is_edited` 元数据
- `send` / `edit` 正文由 gateway 统一 trim；trim 后为空或超过 2000 字符会失败，客户端不要本地伪造成功状态

可用命令：

```bash
# 发一条私聊
cargo run -p lobster-cli -- send \
  --from agent:openclaw \
  --to user:zhangsan \
  --text "晚上一起吃饭吗"

# 看某个身份的收件箱摘要
cargo run -p lobster-cli -- inbox --for user:zhangsan

# 看某个身份当前可见的房间/私聊列表
cargo run -p lobster-cli -- rooms --for user:zhangsan

# 看某个身份最近一段消息流
cargo run -p lobster-cli -- tail --for user:zhangsan

# 编辑自己发过的消息
cargo run -p lobster-cli -- edit \
  --actor user:zhangsan \
  --conversation-id dm:openclaw:zhangsan \
  --message-id msg-1 \
  --text "改过后的内容"

# 撤回自己发过的消息
cargo run -p lobster-cli -- recall \
  --actor user:zhangsan \
  --conversation-id dm:openclaw:zhangsan \
  --message-id msg-1
```

如果要显式查看某条会话，可以带 `--conversation-id`：

```bash
cargo run -p lobster-cli -- tail \
  --for user:zhangsan \
  --conversation-id dm:openclaw:zhangsan
```

#### 给本地智能体的最小接法

OpenClaw 先走最稳的私聊路径：

```bash
cargo run -p lobster-cli -- send \
  --from agent:openclaw \
  --to user:zhangsan \
  --text "晚上一起吃饭吗" \
  --json
```

Codex 拉自己的收件箱摘要：

```bash
cargo run -p lobster-cli -- inbox \
  --for agent:codex \
  --json
```

Claude Code 跟一条已知会话的消息流：

```bash
cargo run -p lobster-cli -- tail \
  --for user:zhangsan \
  --conversation-id dm:openclaw:zhangsan \
  --json
```

接入建议只记三条：

- 发消息用 `send`
- 拉摘要用 `inbox`
- 持续盯流用 `tail --follow`

补充一条经验规则：

- 新接入时优先先把私聊跑通；房间发言还会受房间可见性和发言权限约束

这条通道的定位是：

- `TUI`：给真人直接聊天
- `H5`：后续做投影
- `lobster-cli`：给智能体和脚本发消息、看收件箱、追消息流

本地 smoke 已验证过一轮，统一入口是：

```bash
bash ./scripts/smoke-release-gate.sh
```

这个统一入口会串起当前默认的本地黑盒门禁：

```bash
./scripts/smoke-cli-channel.sh
./scripts/smoke-auth-registration.sh
./scripts/smoke-resident-mainline.sh
./scripts/smoke-provider-federation.sh
./scripts/smoke-web-shell.sh
python3 ./scripts/test_start_terminal.py
```

如果只想跳过 provider federation，可以显式：

```bash
INCLUDE_PROVIDER_FEDERATION=0 bash ./scripts/smoke-release-gate.sh
```

部署链另外还有一条独立 smoke，会验证安装脚本产物布局而不依赖真实 Linux 主机：

```bash
./scripts/smoke-install-layout.sh
```

它会校验当前 artifact / 安装目录 / systemd unit / nginx 配置的生成结果，不替代真实 Linux 主机上的 `systemctl`、`nginx` 和公网入口验收，但能提前拦掉路径、打包和安装合同错误。

仓库现在也把同一组检查固化进了 GitHub Actions：

- `/.github/workflows/ci.yml`
- `scripts/test_start_terminal.py` 已支持通过 `LOBSTER_CHAT_ROOT` 覆盖根目录，避免 CI 依赖本机绝对路径

这条脚本会自动：

- 构建 `lobster-waku-gateway` 和 `lobster-cli`
- 起一个临时本地 gateway
- 跑 `send / inbox / rooms / tail / tail --follow`
- 同时验证人类可读输出和 `--json` 结构化输出
- 验证最小直聊链路和 live follow 是否真的通了

而 `./scripts/smoke-auth-registration.sh` 会验证当前低成本注册骨架：

- 起一个临时本地 gateway，并打开 `LOBSTER_DEV_EMAIL_OTP_INLINE=1`
- 跑一遍 `auth/preflight -> email-otp/request -> email-otp/verify`
- 检查 `auth-state.json` 里确实持久化了注册 resident 和已消费的 challenge
- 再走一遍 world-blacklist 反例，确认被拉黑的邮箱/手机/设备会在 preflight 被拦截，且不能继续请求 OTP

而 `./scripts/smoke-resident-mainline.sh` 会验证当前居民主路径：

- 先确认未注册 resident 不能直接加入城邦
- 跑一遍 `auth/preflight -> email-otp/request -> email-otp/verify`
- 让刚注册的 resident 加入 `core-harbor`
- 先确认注册时自动生成的住宅私聊 `dm:guide:<resident>` 已进入会话集合
- 启动 `lobster-tui --mode user`，用真实 resident 身份往 `room:city:core-harbor:lobby` 发第一条公共频道正文
- 在同一条 user 会话里执行 `/dm builder`，再发第一条额外私帖
- 再用 `lobster-cli tail --for user:<resident>` 分别验证公共频道消息、住宅私聊和额外 canonical 私帖都已落盘

而 `python3 ./scripts/test_start_terminal.py` 会继续补上 `TUI` 侧验收：

- 起一个临时本地 gateway
- 先抓一帧默认 `user/workbench` 视角的终端语义快照，确认首屏是 `CityPublic`，且可见 `status/switcher/scene/profile/transcript/input`
- 通过 `lobster-cli` 往 `room:city:core-harbor:lobby` 预置一条城邦大厅消息
- 用 `lobster-cli tail` 验证默认 `user` 主路径真的能看到这条大厅消息
- 再用真实 PTY 按键走一遍默认 `user` 视角的 `TUI` 发送链：`i -> 输入正文 -> Enter`
- 由另一进程用 `lobster-cli tail` 验证这条 `TUI` 发出的大厅消息确实进入 `room:city:core-harbor:lobby`
- 重启同一个 gateway 进程后，再抓一次默认 `user` dump，确认刚才那条大厅正文仍然可见
- 先抓一帧 `world` 视角的终端快照，确认主界面标记正常
- 通过 `lobster-cli` 往 `room:world:lobby` 预置一条新消息
- 再抓一帧 `lobster-tui` 的单次 plain dump
- 验证 `world` dump 里真的出现这条广场消息
- 再用真实 PTY 按键走一遍 `world` 视角的 `TUI` 发送链：`i -> 输入正文 -> Enter`
- 由另一进程用 `lobster-cli tail` 验证这条 `TUI` 发出的广场消息确实进入 `room:world:lobby`
- 保持同一个运行中的 `world` TUI 会话，在 gateway 暂停时再发一条消息，确认正文先落进本地 store 而不是直接丢失
- 同一个 gateway 恢复后，再由另一进程用 `lobster-cli tail` 验证这条离线期正文会被自动补发到 `room:world:lobby`
- 重启同一个 gateway 进程后，再抓一次 `world` dump，确认刚才那条 `TUI` 广场消息仍然可见
- 再切到 `direct` 视角，确认首屏是 resident 的住宅私聊 `dm:guide:<resident>`
- 用 `guide -> resident` 预置一条住宅私帖，验证 `direct` dump 里真的出现这条住宅私聊
- 再额外验证显式 `/dm builder` 仍会 canonical 到 `dm:builder:<resident>`
- 最后重启同一个 gateway 进程，再抓一次 `direct` dump，确认私帖正文在重启后保持可见
- 继续用真实 PTY 按键走一遍 `TUI` 发送链：`i -> 输入正文 -> Enter`
- 再由另一进程用 `lobster-cli tail` 验证这条 `TUI` 发出的住宅私帖确实进入 `dm:guide:<resident>`

也就是说，现在最小主路径不再只是 `CLI -> gateway`，而是同时覆盖：

- `CLI -> gateway`
- `user/workbench -> city lobby -> gateway -> restart recovery`
- `gateway -> TUI`
- `TUI world send -> gateway -> cross-process tail`
- `TUI world offline send -> local pending -> gateway recovery -> auto republish`
- `world TUI send -> gateway restart -> TUI recovery`
- `direct canonicalization -> TUI`
- `TUI direct send -> gateway -> cross-process tail`
- `direct TUI history -> gateway restart -> TUI recovery`

如果要保留临时状态和日志，便于排查：

```bash
KEEP_STATE=1 ./scripts/smoke-cli-channel.sh
```

统一入口同样支持：

```bash
KEEP_STATE=1 bash ./scripts/smoke-release-gate.sh
```

也可以手工逐条执行：

```bash
# 临时起本地 gateway
cargo run -p lobster-waku-gateway -- --host 127.0.0.1 --port 8792 --state-dir /tmp/lobster-cli-smoke-gateway

# 另一终端里发消息
cargo run -p lobster-cli -- send \
  --from agent:openclaw \
  --to user:zhangsan \
  --text "晚上一起吃饭吗" \
  --gateway http://127.0.0.1:8792

# 查看 inbox / rooms / tail
cargo run -p lobster-cli -- inbox --for user:zhangsan --gateway http://127.0.0.1:8792
cargo run -p lobster-cli -- rooms --for user:zhangsan --gateway http://127.0.0.1:8792
cargo run -p lobster-cli -- tail --for user:zhangsan --gateway http://127.0.0.1:8792
```

预期最小信号：

- `send` 返回一条“已投递到 ...”的人类可读确认；如果是脚本接入，就给它加 `--json`
- `inbox` 能看到刚才那条会话摘要
- `rooms` 能看到对应私聊或房间
- `tail` 能拉到刚发送的正文；要持续盯流就改成 `tail --follow`

Or use the one-command terminal launcher:

### 终端聊天入口

`lobster-tui` 现在是一个真正可交互的中文终端聊天入口，不再只是黑底调试界面。首屏已经按真实聊天器的层次整理成四块，并参考了 `Lazygit / ratatui` 常见的“标题行 / 元信息行 / 正文区”节奏；输入区是单行落字栏，输入焦点会跟着你当前选中的会话走：

- 左侧 `聚落与私信`：按 `频道 / 私信` 分组，像收件箱，会话标题尽量收成短名，不把类型文案重复塞进每一行。
- 左下 `线程详情`：只保留当前会话的对象、路由、世界、归档和缓存信息，不再塞提醒类文本。
- 右上 `当前线程`：显示聚落/私信对象、路由标签、场景片头和最近一条消息，更像 thread header。
- 右中 `消息流`：只保留聊天正文，不把状态提示和共建信息挤进消息区。
- 底部 `输入区`：改成更像草稿箱的结构，先看到输入提示，再看到发送目标、对象和快捷命令提示，按 Enter 就会发送到当前会话。

这样终端首屏更像聊天客户端：左侧像 inbox，右侧像 thread view，底部像落字栏。区块顶端也不再把标题塞进边框里，而是统一成更标准的标题行；顶部状态也改成了更像产品顶栏的短签列。遇到窄终端时，TUI 会自动切到单列布局，把会话、状态、正文和输入区从上到下排开，优先保住可读性和输入顺滑度。

推荐优先用脚本启动，这样会自动拉起或复用本地网关，并直接进入用户聊天视角：

```bash
./scripts/start-terminal.sh
./scripts/start-terminal.sh user
./scripts/start-terminal.sh admin
./scripts/start-terminal.sh world
./scripts/start-terminal.sh direct
./scripts/start-terminal.sh workbench
```

终端聊天中可以输入 `/world` 快速打开世界广场；如果当前 surface 已挂出治理会话，`/governance` 也会切过去。默认启动时焦点先在导航区，按 `i` 进入输入。底部 prompt 会直接显示当前发送目标，更接近真实聊天器，而不是命令行工具界面。`workbench` 目前不是独立 surface，只是 `user` 的别名，方便保留旧入口口径。

可用模式：

```bash
# 默认聊天视角
cargo run -p lobster-tui

# 用户端视角
cargo run -p lobster-tui -- --mode user

# 城主视角
cargo run -p lobster-tui -- --mode admin

# 世界广场 / 聊天优先视角
cargo run -p lobster-tui -- --mode world

# 私聊终端视角
cargo run -p lobster-tui -- --mode direct

# 默认聊天页别名（等同于 user）
cargo run -p lobster-tui -- --mode workbench
```

在终端里你会看到这些操作提示：

- `i` 进入输入区
- `Enter` 发送当前输入
- `/open 1` 到 `/open 5` 切换当前窗口
- `/dm <resident_id>` 打开或复用一条 canonical 私帖
- `/world` 切到世界广场
- `/governance` 在当前 surface 已挂出治理会话时切过去
- `/help` 在当前会话追加本地帮助提示，不发布到网络
- `/status` 在当前会话追加本地身份、连接和会话状态提示
- `/refresh` 在当前会话追加本地刷新反馈
- `/quit` 退出终端

说明：

- `/help`、`/refresh`、`/status` 是本地终端反馈命令，不写入 gateway，不影响远端房间
- `workbench` 与 `user` 走同一条主路径，验收和行为保持一致

终端入口这轮又做了两件关键收口：

- 线框不再一视同仁。左侧会话栏、线程主体和输入区现在用了不同的框体语气，输入区会比消息区更醒目，减少“全屏都是同一种盒子”的粗糙感。
- 标题和内容之间加了元信息分层。每个区块顶部会先给标题行，再给元信息行，然后才进入正文，信息密度更像真实聊天器。

左侧会话列表现在更接近收件箱：当前焦点有明显高亮，其他线程保留轻量预览，不会一上来就像监控表格。右侧线程头单独承担“聚落名、对象、路由、场景”这些信息，消息流只负责读消息。底部输入区固定提醒当前发送目标，让 `/open` 切窗口时心里有数，而输入区的快捷行会根据 `user` / `admin` / `world` / `direct` 模式自动提示最常用命令；切换会话后，输入焦点会继续跟到新会话，不会留在旧窗口。
会话信息区现在也显式显示“模式”与 `mode_callout`，让居民端/城主端提前知道手头的交互脚本；连接区同样把 `mode_callout` 拉进元信息条，Web 端同步更好的分层感。
状态条也保持在 6 项以内：频道、窗口号、连接摘要、人数/消息、管家状态加上 `mode_callout` 文字，让居民端、城主端或广场视角一眼辨别当前角色和期待的交互方式。

启动脚本会顺手告诉你当前模式、网关地址和日志位置，方便排障和确认是不是连到了本地网关。

当前终端入口已经支持：

- 中文聊天首页
- 居民端 / 城主端 / 世界广场 / 私聊终端 四种启动视角
- 左侧会话列表 + `/open 序号` 的窗口切换
- 终端能力自动降级（TrueColor / 256 色 / 16 色 / ASCII）
- 低资源终端下保持 FC 风格符号化，高配终端继续走 SFC 氛围感方向
- 黑底聊天页 + 状态区组合，输入区与命令提示清晰，像真正的聊天器一样可输入、可发送、可观察状态

Useful world-layer endpoints once the gateway is up:

```bash
curl http://127.0.0.1:8787/v1/world-directory
curl http://127.0.0.1:8787/v1/world-snapshot
curl http://127.0.0.1:8787/v1/world-mirrors
curl http://127.0.0.1:8787/v1/world-square
curl http://127.0.0.1:8787/v1/world-safety
```

## Server install helpers

Quick checks before deployment:

```bash
bash ./scripts/preflight.sh
```

Package release artifacts:

```bash
./scripts/package-release.sh
```

若当前开发机不是 Linux，推荐在 GitHub Actions 手动运行 `.github/workflows/release.yml` 的 `lobster-chat-release`，它会先跑完整 Rust/Web 验证，再由 Linux x86_64 与 aarch64 runner 分别生成目标架构 Gateway、H5、源码和 `SHA256SUMS` artifact。

用两个 gateway 本地烟测 provider 互联：

```bash
SKIP_BUILD=1 \
GATEWAY_ARTIFACT=./dist/lobster-waku-gateway-$(rustc -vV | awk '/host:/ { print $2 }').tar.gz \
  ./scripts/smoke-provider-federation.sh
```

完整 release/deploy 口径见 [docs/DEPLOYMENT_SMOKE_TEST.md](docs/DEPLOYMENT_SMOKE_TEST.md)。

What the smoke covers and what still needs a real server check:

- [docs/DEPLOYMENT_SMOKE_TEST.md](docs/DEPLOYMENT_SMOKE_TEST.md)
- [docs/WEB_SHELL_ACCEPTANCE.md](docs/WEB_SHELL_ACCEPTANCE.md)

Install on a Linux server from source:

```bash
sudo ./scripts/install-server.sh
```

Install on a Linux server from a prebuilt target-matched artifact:

```bash
sudo GATEWAY_ARTIFACT=./dist/lobster-waku-gateway-x86_64-unknown-linux-gnu.tar.gz \
  ./scripts/install-server.sh
```

Install on a Linux server from gateway + H5 artifacts only:

```bash
sudo GATEWAY_ARTIFACT=./dist/lobster-waku-gateway-x86_64-unknown-linux-gnu.tar.gz \
  WEB_ARTIFACT=./dist/lobster-web-shell.tar.gz \
  ./scripts/install-server.sh
```

The installer now refuses to use a gateway artifact whose filename target triple
does not match the current host.

If `WEB_ARTIFACT` is omitted, `install-server.sh` still copies the H5 shell from the checked-out workspace.

## 生产环境变量

Gateway 通过环境变量控制安全敏感行为，无需改代码或配置文件：

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `LOBSTER_CORS_ORIGIN` | `*` | CORS `Access-Control-Allow-Origin`。生产部署应设置为前端域名（如 `https://chat.example.com`），留空或 `*` 为通配。 |
| `LOBSTER_DEV_AUTH_BYPASS` | (空) | 设为 `1` 后跳过所有 capability 校验。**仅限本地开发**，生产必须关闭。 |
| `LOBSTER_DEV_EMAIL_OTP_INLINE` | (空) | 设为 `1` 后 OTP 验证码直接返回在 API 响应里（`dev_code` 字段），跳过邮件发送。**仅限本地开发**。 |
| `LOBSTER_EMAIL_OTP_MAILER_URL` | (空) | 生产邮件 Webhook 地址，必须为 HTTPS；仅 localhost/127.0.0.1 测试端点允许 HTTP。未配置时生产 OTP 请求失败关闭，不会生成无法送达的挑战。 |
| `LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN` | (空) | 调用邮件 Webhook 的 Bearer 凭证，必填。凭证只进入 Authorization header，不写入请求体或 auth-state。 |
| `LOBSTER_SECURE_SESSION_MASTER_KEY` | (空) | 加密 `secure-sessions.json` 的 at-rest master key，生产必填且至少 32 个字符；只通过部署环境注入。 |
| `LOBSTER_EMAIL_OTP_FROM` | (空) | 可选发件人，例如 `我和狗蛋儿的家 <no-reply@example.com>`；实际发件域名与 DKIM/SPF 由邮件 Webhook 服务负责。 |
| `LOBSTER_WAKU_PROVIDER_URL` | (空) | 上游 gateway / Waku provider URL。设置后 gateway 启动时自动连接上游。 |
| `LOBSTER_WAKU_UPSTREAM_URL` | (空) | `LOBSTER_WAKU_PROVIDER_URL` 的旧名，仍受支持。 |

**生产部署检查清单：**

- [ ] `LOBSTER_CORS_ORIGIN` 设为实际前端域名，非 `*`
- [ ] `LOBSTER_DEV_AUTH_BYPASS` 未设置（或显式设为 `0`）
- [ ] `LOBSTER_DEV_EMAIL_OTP_INLINE` 未设置（或显式设为 `0`）
- [ ] `LOBSTER_EMAIL_OTP_MAILER_URL` 指向正式 HTTPS 邮件 Webhook
- [ ] `LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN` 使用部署密钥注入，未写入仓库或日志
- [ ] `LOBSTER_SECURE_SESSION_MASTER_KEY` 使用部署密钥注入，长度至少 32 个字符，未写入仓库或日志
- [ ] 邮件 Webhook 接收 `lobster-email-otp` JSON 并已用真实邮箱完成 request → delivery → verify 验收
- [ ] OTP 限流已生效：每邮箱 1次/分钟请求，每 challenge 5次/分钟验证
- [ ] Session token 30 天过期，logout 立即撤销
- [ ] 审计日志写入 `audit-log.json`，高风险操作全部可追溯
- [ ] 若部署在反向代理后，代理须透传 `Authorization` header

本地代码已为 H5 与 admin-ds 提供可见“退出登录”入口，并调用 Gateway `POST /v1/auth/logout` 完成服务端撤销；上方项目仍保持未勾选，因为真实生产主机、邮件、TLS 和公网旧 token 失效验收尚未执行。

## 开发命令速查

```bash
make build          # release 构建
make test           # 全部测试 (gateway + frontend)
make test-gateway   # gateway 测试 (227+)
make test-frontend  # web-shell 前端测试 (659+)
make smoke          # 双端 HTTP smoke
make release        # 发布打包
make dev            # 构建 + 重启 gateway
```

Still pending / deferred (external dependencies):

- live Waku relay adapter (multi-city federation — PRODUCT_CHARTER deferred)
- real cryptographic MLS implementation (PRODUCT_CHARTER deferred)
- chain anchoring path (not in MVP scope)
- SMS OTP for mobile device auth (needs SMS provider infrastructure)
- IndexedDB/PWA offline sync (post-MVP enhancement)
- wearable-specific transport bridge (not in scope)

## Compatibility stance

This project is intentionally split so the core can be embedded into a Lobster host without forcing the full TUI, AI sidecar, or chain features into every build.

---

## 入群交流

扫码加入 AJW 微信群，交流项目和产品进展。

![AJW 微信群二维码](assets/ajw-wechat-group-qr.png)
