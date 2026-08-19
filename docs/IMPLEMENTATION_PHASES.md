# Implementation Phases

## 当前交付顺序（2026-04-15 重置）

当前主线按下面顺序推进。2026-05-19 起，正式上线目标按“先真实可用，再交互完善，再鲁棒性和可维护性补齐”的顺序执行，任何新界面都不得绕过 gateway 合同成为私有真源：

1. 先补厚 `conversation_shell / scene_render` 这套正式合同
2. 由 `H5` 承接当前主交互入口与 `SFC` 场景主路径
3. 同步推进 `TUI` 对同一合同的并行映射
4. 补齐 `lobster-cli` 这条给本地智能体复用的命令行聊天通道
5. 再进入真实 transport 评估与接入
6. 再进入真实加密落地
7. 眼镜端后置

因此，下面这些内容目前属于“协议与架构预留”，不是当前 `MVP` 的完成标准：

- `World Square`
- `World Safety`
- 多城邦互联产品面
- H5 离线/PWA 完整体验
- 穿戴设备专用交互
- 装扮编辑器、素材系统、自由摆放

当前 `MVP` 的完成标准应收敛到：

- 单城邦范围内的私聊、群聊、房间聊天主路径
- 单城邦内部先用中心化 gateway 跑通身份、居民目录、在线状态、离线消息、房间配置和维护审计
- `H5` 主交互可用
- `TUI` 对同一正式合同的映射可用
- `conversation_shell / scene_render` 成为稳定的正式合同
- `lobster-cli` 可供 `OpenClaw / Codex / Claude Code` 等智能体直接复用
- 为真实 transport 与真实加密保留后续接入边界

## 正式上线开发先后顺序（2026-05-19）

### P0: 单城邦中心化 IM 真闭环 (2026-05-25 后端侧已验收)

1. 身份与会话可见性
   - 邮箱 OTP 登录、bearer 会话、访客只读/禁发、token 失效回登录态。✅
   - `GET /v1/shell/state?resident_id=<id>` 必须只返回当前居民可见私聊、公共房间和系统会话。✅
2. 真实消息链路
   - send/edit/recall/retry/export 全部走 gateway，H5 不伪造成功态。✅
   - pending echo、失败重发、committed copy 去重、断线恢复、SSE 刷新必须有自动化 smoke。✅
3. 居民目录与个人房间入口
   - 左栏作为居民/会话栏，支持搜索、未读、最近消息、居民头像入口。✅ (后端侧: GET /v1/residents?q=, enrich 含 online/last_seen/avatar)
   - 点击居民头像先确认，再进入对方个人房间私聊。✅ (POST /v1/direct/open)
4. 公共房间
   - 单城主城公共房间、世界广场只读入口、公告/讨论/跨城发现摘要清晰分层。✅
5. 在线状态与未读 (2026-05-25 新增)
   - `POST /v1/shell/presence` heartbeat，120s 阈值在线判定。✅
   - `POST /v1/shell/read` 标记已读清零。✅
   - `ShellRoomState.unread_count` 字段推送前端。✅
6. 管理后台系统状态 (2026-05-25 新增)
   - `GET /v1/admin/summary` 返回居民/房间/消息计数、在线数、运行时长、状态版本。✅

### P1: 空间房间与交互完善

1. Gateway-owned room config
   - `scene_render.scenes[]` 输出 `image_layer` 和 `hotspot_layer`。
   - H5 渲染这两层；TUI/眼镜端降级读取同一配置。
2. 房间编辑器 MVP
   - 仅允许编辑图像层 preset、热点标签、热点坐标和交互说明。
   - 保存路径必须是 gateway 配置；H5 只做草稿/表单态。
3. 场景交互验收
   - 热点默认不遮挡，hover/focus/靠近显示，清屏模式能显示选择提示。
   - 桌面/移动端输入区、左栏、舞台尺寸稳定，不因文本或窗口比例错位；编辑器以 16:9 `scene-canvas` 作为背景与热点共同坐标系，已通过移动 viewport bounding-box 验收。

### P2: 后台与运维可用

1. DeepSeek 版 `admin-ds.html` 作为正式后台视觉基线。
2. 后台只读 projection 先接入，再逐步开放安全的管理操作。
3. 管理操作必须有权限、禁用原因、审计提示和失败反馈。

### P3: 技术债压降与工程鲁棒性

1. 按 `FRONTEND_DEBT_REDUCTION_PLAN.md` 拆 `app.js`：scene/hotspot、room projection、message action、composer、auth、governance。
2. CSS 按 tokens/base/scene/chat/world/admin 拆分；页面只加载必要样式。
3. Web/Gateway/TUI/CLI 测试进入固定验收命令，真实浏览器 smoke 覆盖核心路径。

### P4: TUI/CLI parity

1. TUI 与 CLI 映射同一 `conversation_shell / scene_render` 合同。
2. `/help`、`/status`、`/refresh` 等本地命令不得写 gateway。
3. edit/recall/send/export 状态与 H5 语义一致。

### P5: 真实 transport、加密与跨城

1. 单城中心化稳定后，再接真实 Waku/远程 gateway。2026-08-13 已完成限时开源调研并批准隔离 P5.1；transport PoC 采用 `Logos Delivery node + loopback REST sidecar`。当前只有默认关闭、尚未验收的本地代码 WIP，未部署节点、未修改生产。
2. MLS/加密按明确边界推进，不阻塞单城 IM。首选用 OpenMLS 做受控 PoC，`mls-rs` 保留为备选；现有 AES-GCM skeleton 不得称为标准 MLS。
3. 首发范围默认为 DM/显式私密房间，公共城市房间保持服务器可治理模式；安全房间禁止静默回退明文。
4. P5 按 P5.0 合同审批 → P5.1 双 Waku 节点 lab → P5.2 native MLS → P5.3 H5 WASM/TUI 互操作 → P5.4 shadow canary → P5.5 私密房间 opt-in 推进。
5. 详细决策、威胁边界、退出条件和回滚门见 [`docs/adr/0001-p5-native-waku-and-standard-mls.md`](adr/0001-p5-native-waku-and-standard-mls.md)。Atlas Proposal 已批准 P5.1 隔离实验；P5.2、生产基础设施与切换仍需各自门禁。
6. P5.1 暂停点：官方 master `23b0d31e848812ad54f5d5f390854cb8dd26fe89` 的节点构建因 GitHub TLS EOF 未完成；adapter 尚未接入 `lib.rs`，未跑 feature/workspace 测试，也未执行双节点 100+100、重启/Store 恢复与日志脱敏验收。恢复时必须从这些未完成项继续，不能把 WIP 计为 native Waku 已落地。

## Phase 1: Skeleton

- Rust workspace
- domain model
- host adapter contracts
- local archive contracts
- standalone TUI runtime

## Phase 2: Local-first messaging

- local room state
- local message append path
- archive policy
- file-backed persistence snapshot
- room timeline rendering
- resident directory/search projection for single-city IM
- resident avatar as personal-room/private-chat entry with confirmation before switching rooms
- embedded / mobile-web / wearable host split baked into interfaces

## Phase 3: Waku light transport

- content topic mapping
- light push send path
- filter-based receive path
- store-based recent history recovery
- low-resource subscription strategy for embedded, mobile-web, and wearable hosts
- gateway-backed adapter boundary first
- in-memory gateway for development and offline tests
- JSON-friendly gateway request / response contract for sidecars and remote adapters
- localhost gateway state persistence for restart-safe resident sessions
- localhost world/city coordination endpoints for membership and public-room state across settlements
- localhost resident directory and direct-message bootstrap endpoints
- upstream gateway interconnect for multi-city discovery experiments
- provider-status endpoint for real Waku provider / remote gateway integration
- provider connect / disconnect endpoints with persisted upstream bridge config
- world snapshot bundle endpoint with checksum metadata for mirror-city sync and cached projections
- real network adapter second
- native adapter candidate: Logos Delivery node official REST API, loopback-only and feature-gated
- versioned protobuf envelope with opaque bucketed content topics; never place resident/conversation identifiers in topics
- Gateway delivery ledger remains authoritative; Light Push acknowledgement and Waku Store are not final-delivery/canonical-state proofs

## Phase 4: MLS security

- room bootstrap
- 1v1 conversation bootstrap
- persisted 1v1 session skeleton state via gateway
- encrypted payload boundary
- epoch management
- RFC 9420 adapter candidate: OpenMLS first, mls-rs fallback; custom AES-GCM skeleton remains migration-only
- MLS credentials are per-device and bound by the existing resident authentication service
- application plaintext and epoch secrets remain device-local; Gateway stores only encrypted envelopes and delivery/auth metadata
- first rollout is opt-in DM/private rooms, with no plaintext fallback and no server-side search/moderation/AI claim

## Phase 5: AI sidecar

- translation hooks
- summarization hooks
- semantic search hooks
- fully optional runtime
- mobile-web friendly streamed responses
- wearable-friendly compact answer shaping
- OpenClaw-powered room and city helper slots
- caretaker message boards for personal rooms
- decoration helpers for room and city scene edits
- merchant/listing helpers for room storefront experiments

## Phase 6: Chain anchoring

- identity anchor
- device root updates
- conversation / room state anchors
- batched message hash anchoring
- optional stake / anti-spam rules

## Cross-cutting world and city rules

- World / City / Resident portability model
- city-lord powers strong enough for public settlement stewardship
- private 1v1 plaintext kept out of city-lord authority
- city creation, resident join, and public-room bootstrap exposed through the localhost gateway
- H5 projection participates in world and city upkeep as a resident-facing follow surface rather than just a passive timeline viewer
- world-directory discovery should be mirrored across multiple cities
- a World Square should exist as a cross-city public commons
- a world safety workflow should be able to quarantine malicious cities, process public safety reports, and publish deny lists without taking ownership of private-message plaintext
- in severe confirmed cases, the world layer can now revoke a resident identity's portability and distribute resident-sanction plus hashed registration blacklist entries
- a low-cost auth skeleton now exists for email OTP registration/login with mobile kept as a hashed anti-abuse handle
- the H5 follow projection can now drive that auth skeleton through an email OTP request + verify flow
- the H5 follow projection now has a resident-facing public-abuse report path into the world safety queue
- server install and release packaging should be scripted rather than relying on developer memory

## Cross-cutting rendering

- desktop TUI follows an SFC-inspired palette philosophy when terminal capability allows it
- low-end or headless terminals degrade to FC-style symbolic rendering
- color, glyph, and portrait capability are runtime-detected rather than hard-coded
- themed nomenclature is planned as a presentation layer over neutral protocol nouns
- city scenes and personal room scenes are planned as metadata-driven spatial surfaces over the same neutral chat/shared-rules core
- personal room scenes should expose two configurable layers: `image_layer` for the composed room image and `hotspot_layer` for normalized interaction regions
- H5 can edit/render those layers, but gateway room configuration remains the source of truth and TUI/wearable clients must degrade the same config
- user identity in rooms should use personalized pixel avatars rather than a fixed lobster mascot
- room and city scenes should be able to host visible helper-bot slots without forcing OpenClaw into every deployment
