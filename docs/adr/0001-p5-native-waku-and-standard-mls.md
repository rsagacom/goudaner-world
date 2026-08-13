# ADR-0001: P5 原生 Waku 与标准 MLS 接入候选

- 状态：**Proposed（待 Atlas Proposal 审批，不代表已采纳）**
- 日期：2026-08-13
- Atlas Proposal：`proposal.38c8236eb16492f31a94`（pending）
- 范围：开源调研、架构边界、最小验证计划
- 非范围：引入依赖、修改生产协议、开通新基础设施、生产发布

## 1. 结论摘要

P5 推荐按两个彼此解耦的适配层推进：

1. transport 首选运行真实 `logos-messaging/logos-delivery` 节点，并由 Gateway 通过仅监听 loopback 的官方 REST API 接入；不让 H5 首阶段直接承担 P2P 节点职责，也不采用 C FFI 嵌入 Gateway。
2. MLS 首选以 `OpenMLS 0.8.1+` 做受控 PoC；`mls-rs 0.55.x` 保留为互操作性和浏览器可行性备选。只有 native、H5 WASM、重启、离线和多设备移除矩阵全部通过后，才提交依赖采纳决策。
3. E2EE 首阶段只覆盖直接消息和显式私密房间。公共城市房间继续沿用当前可治理的服务器可读模式，避免把搜索、治理和 AI 能力静默做成失效按钮。
4. Gateway 继续是身份、权限、目录、投递账本和密文信封的唯一服务端真源；MLS 明文和 epoch secret 必须只存在于成员设备，不能再由 Gateway 持有或备份。
5. 现有 `transport-waku` 是 HTTP federation/in-memory 适配层，现有 `crypto-mls` 是自研 AES-GCM/HKDF 骨架。二者只能作为迁移接口和测试夹具，不能更名包装为原生 Waku 或 RFC 9420 MLS。

该结论只批准最小实验路线，不批准生产切换。

## 2. 当前实现真相

| 领域 | 当前代码 | 与目标的差距 |
| --- | --- | --- |
| transport | `crates/transport-waku` 通过 `HttpWakuGatewayClient` 调用远端 `/v1/waku`，或使用内存实现 | 没有运行 Waku Relay / Light Push / Filter / Store 协议 |
| wire format | `WakuFrameCodec` 用 postcard 编码完整 `MessageEnvelope` | 不是面向跨实现兼容的版本化 protobuf；当前 envelope 可含明文 |
| topic | content topic 直接包含 `conversation_id` | Filter / Store / Light Push 对端可观察 topic，直接 ID 会泄露会话关系元数据 |
| crypto | `crates/crypto-mls` 使用自研 AES-256-GCM、HKDF 和服务端持久化 group key | 不包含 RFC 9420 的 KeyPackage、Welcome、Commit、epoch 和成员移除协议 |
| server features | Gateway 执行服务端搜索、导出、治理、编辑/撤回和 AI hooks | 真 E2EE 下 Gateway 不再拥有明文，必须明确功能策略而不是假装保持完全等价 |

## 3. 上游候选矩阵

评分为 1（弱）到 5（强）；“推荐”表示进入 PoC，不表示生产采纳。

### 3.1 Waku transport

| 候选 | 协议真实性 | 当前架构适配 | 运维可控 | 主要风险 | 结论 |
| --- | ---: | ---: | ---: | --- | --- |
| Logos Delivery 节点 + REST sidecar | 5 | 5 | 4 | 需要独立节点资源；事件读取有消费/淘汰语义，必须实现持久化游标、去重和恢复 | **推荐** |
| `logos-delivery-js` 直接进入 H5 | 5 | 2 | 2 | 浏览器后台存活、移动网络、P2P 元数据和本地状态复杂度过早压到主入口 | 首阶段不采用 |
| C FFI / libwaku 嵌入 Rust Gateway | 5 | 2 | 2 | ABI、内存安全、交叉编译和升级边界显著扩大 | 不采用 |
| 现有 `/v1/waku` HTTP federation | 1 | 5 | 5 | 不运行 Waku 协议 | 仅保留兼容基线 |

选择 REST sidecar 的原因是：它复用现有 transport adapter 边界，同时让真实 Waku 节点与 Gateway 的故障域、升级节奏和资源预算分离。REST 与 admin 接口只允许 loopback；公网节点只暴露 Waku 所需端口。

### 3.2 MLS

| 候选 | RFC 9420 | 独立安全审计 | Rust/存储适配 | H5/WASM | 结论 |
| --- | ---: | ---: | ---: | ---: | --- |
| OpenMLS `0.8.1+` | 5 | 4 | 5 | 2 | **首选 PoC**；WASM binding 仍属实验性，必须自建最小 worker 并实测 |
| `awslabs/mls-rs 0.55.x` | 5 | 2 | 5 | 4 | 备选；WASM 流程更明确，但项目公开说明尚无完整第三方安全审计，WebCrypto provider 仍实验性 |
| 当前 `crypto-mls` skeleton | 1 | 1 | 5 | 1 | 仅作为接口迁移源和负向测试，不进入标准 MLS 路径 |

OpenMLS 的优先级来自 RFC 9420 实现、可替换的 crypto/storage provider 以及已公开的独立审计与修复记录。正式固定版本前必须重新核对全部 advisory、审计遗留项、浏览器随机数/时间来源和禁用功能；不得仅因仓库无公开 advisory 就推断实现安全。

## 4. 目标边界

```text
H5 Web Worker / TUI device
  └─ MLS engine + device-local protected state
       └─ versioned opaque envelope
            └─ Gateway: auth / policy / routing / encrypted delivery ledger
                 └─ loopback REST adapter
                      └─ Logos Delivery node
                           └─ Waku network
```

### 4.1 身份和设备

- 邮箱 OTP 继续确认 resident account，但不直接等同于 MLS 设备凭据。
- 每台设备生成独立签名密钥并注册为独立 MLS leaf；服务端 Authentication Service 绑定 `resident_id`、device id 和 credential fingerprint。
- KeyPackage 必须有使用次数、过期时间、撤销和补货状态；Welcome 只能投递给目标设备。
- 设备丢失时执行 remove proposal/commit。禁止以“从服务器导出群密钥”作为恢复方案。

### 4.2 Delivery Service

- Gateway/Waku 是 RFC 9420 语义下不可信 Delivery Service：可以观察投递时间、大小和 opaque routing token，但不能读取应用正文。
- Gateway 持久化 KeyPackage 索引、Welcome/Commit/application 密文、投递状态和幂等键；Waku Store 只做恢复来源，不是唯一事实源。
- Waku 允许重复、乱序和短暂不可达。信封至少包含版本、opaque group token、epoch、sender leaf、client sequence、message id、payload hash、TTL 和 message kind。
- 发送成功必须区分“sidecar 已接受”“某远端 peer 已接收”和“目标设备已确认”；Light Push acknowledgement 不能包装成最终送达回执。

### 4.3 topic 和元数据

- 禁止把 resident id、邮箱、用户名、房间名或原始 `conversation_id` 放入 content topic。
- topic 使用固定应用命名和有限数量的 opaque bucket，例如 `/<application>/1/messages/proto` 或按城市随机 bucket 分片；真实 group token 位于加密/最小暴露信封内。
- protobuf schema 必须显式版本化；postcard 只留在本地兼容测试，不作为公网跨实现合同。
- 日志不得记录 application payload、Welcome、KeyPackage 私有材料、epoch secret 或完整 credential；指标只记录计数、延迟、状态码和脱敏 bucket。

### 4.4 产品能力策略

| 能力 | 公共城市房间（首阶段） | MLS 私聊/私密房间 |
| --- | --- | --- |
| Gateway 正文搜索/AI/内容治理 | 保持现状 | 禁用，不发送明文 |
| 客户端本地搜索/导出 | 可用 | 解密后在设备本地执行 |
| 编辑/撤回 | 当前 Gateway 语义 | 变为不可变密文后的 MLS control message，客户端按权限应用 |
| 新设备补历史 | 当前服务端投影 | 默认只读加入后消息；历史共享必须单独设计并显式告知成员 |
| 备份恢复 | 服务端状态备份 | 不备份 epoch secret；只允许明确设计的设备迁移/恢复机制 |
| 降级 | 现有模式 | fail-closed；安全房间不得静默回退明文 |

H5 的 MLS 状态先由专用 Web Worker 隔离，PoC 只使用临时状态。生产化前必须完成 XSS/CSP 审计、加密的 IndexedDB storage provider、不可导出 wrapping key 可行性和设备迁移 UX。普通 Gateway tar 备份必须明确排除 MLS 客户端 secret，否则会破坏 forward secrecy 的删除要求。

## 5. 可执行阶段

### P5.0：合同冻结与实验门（本 ADR）

- 交付：ADR、Atlas pending Proposal、功能边界和停止条件。
- 退出：文档脱敏/链接/格式检查通过；用户明确批准 Proposal 后才能改依赖。
- 停止：未确定公共房间与私密房间策略、设备恢复策略或实验节点资源时，不进入生产设计。

### P5.1：真实 Waku 双节点 lab（2–3 个工作日）

- 在隔离环境运行两台 Logos Delivery 节点；不与当前生产主机争用资源。
- 在 `transport-waku` 新增 feature-gated REST adapter，默认关闭；保持现有 adapter 可回滚。
- 实现版本化 protobuf envelope、固定 opaque topics、send/receive event polling、Store recovery、幂等和持久化游标。
- 验收：A→B、B→A 各 100 条；重复/乱序无重复投影；sidecar 重启后从 Gateway ledger + Store 收敛；日志 secret scan 通过。
- 停止：丢消息无法由 ledger 恢复、topic 暴露业务 ID、REST 绑定非 loopback 或节点资源不可控。

### P5.2：OpenMLS native PoC（3–5 个工作日）

- 新建标准 MLS adapter，不替换 skeleton；先只覆盖两个 native 测试设备。
- 跑通 KeyPackage 发布、建组、Welcome、Commit、PrivateMessage、成员移除、epoch 前进和乱序恢复。
- Gateway 断言只看到密文；通过负向测试证明被移除设备不能解密新 epoch。
- 验收：重启前后消息连续；旧 epoch 删除策略可验证；fuzz/恶意载荷不导致 panic；依赖许可和 advisory 清单固定。
- 停止：任何密钥写入 Gateway、无法可靠删除旧 secret、或所需 OpenMLS 功能存在未接受的审计风险。

### P5.3：H5 WASM 与 TUI 互操作（4–7 个工作日）

- 用同一 adapter contract 构建 H5 Web Worker/WASM 和 native TUI；不复制协议逻辑到 JavaScript。
- 三设备矩阵：两个独立浏览器 context + 一个 TUI，覆盖加入、离线、重启、多设备、remove/re-add。
- 验收：跨端 transcript hash 一致；Gateway 无明文；CSP/XSS 回归、storage migration 和 quota failure 均 fail-closed。
- 停止：浏览器 state 损坏导致静默明文降级，或 WASM binding 需要维护不可接受的私有 fork。

### P5.4：transport shadow canary（3–5 个工作日）

- 只复制脱敏测试信封到 Waku，不复制真实居民明文；默认 feature flags 为 off。
- 对比现有 HTTP federation 与 Waku 的接收率、P95、重连和资源曲线。
- 验收：连续观测窗口无数据分叉；关闭 flag 即完全回到旧 transport；无生产协议不可逆迁移。

### P5.5：私密房间小流量 opt-in（5–10 个工作日）

- 仅在新建 DM/私密房间启用；UI 显示成员设备、验证状态、恢复边界和不可用功能。
- 先内部账号，再受控居民；公共房间保持原路径。
- 验收：跨设备消息、离线补偿、成员移除、设备丢失和完整回滚演练全部通过，才讨论扩大范围。

## 6. 基础设施与发布门

- Waku 官方部署资料建议为节点准备至少 2 GB 内存，当前生产节点资源低于该建议基线；P5.1 使用独立实验节点，不能直接共置生产。
- node/admin REST 必须 loopback-only；禁止为省事暴露公网管理面。
- 运行时默认 `LOBSTER_NATIVE_WAKU=0`、`LOBSTER_STANDARD_MLS=0`。只有明确 canary allowlist 才能开启。
- transport 和 MLS 各自可独立关闭；数据库 schema 采用 additive migration，旧客户端仍能识别“安全房间不可用”而不是尝试明文发送。
- 任一生产步骤仍需单独 Atlas task/lease、基础设施 Proposal、备份、回滚和真实端到端验收。

## 7. 待审批默认值

1. MLS 首发范围：仅 DM/显式私密房间；公共房间不变。
2. transport：Logos Delivery REST sidecar，独立实验节点。
3. MLS：OpenMLS 优先 PoC，mls-rs 做备选对照。
4. 历史：新设备默认不自动获得加入前历史。
5. 恢复：先做设备到设备迁移；不提供服务端群密钥托管。

这些值须经 Atlas Proposal 批准后才成为实施决策。

## 8. 一手资料

- [Waku protocol overview](https://docs.waku.org/learn/concepts/protocols/)
- [Waku content topics and metadata guidance](https://docs.waku.org/learn/concepts/content-topics/)
- [Waku node deployment requirements](https://docs.waku.org/run-node/run-docker/)
- [Logos Delivery node repository](https://github.com/logos-messaging/logos-delivery)
- [Logos Delivery REST API repository](https://github.com/logos-messaging/logos-delivery-rest-api)
- [RFC 9420: The Messaging Layer Security Protocol](https://www.rfc-editor.org/rfc/rfc9420)
- [OpenMLS repository](https://github.com/openmls/openmls)
- [OpenMLS project security/audit updates](https://blog.openmls.tech/)
- [mls-rs repository](https://github.com/awslabs/mls-rs)
