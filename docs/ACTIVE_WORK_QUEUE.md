# lobster-chat Active Work Queue

Last updated: 2026-08-13

> 说明：下方按日期排列的记录保留当时的交接背景；如与本页最新日期区块冲突，以最新区块和 `docs/DEPLOYMENT.md` 为准。

## 2026-08-13 公网生产状态只读复核

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 入口可达性 | 已确认 | `https://chat.ajw.cn/health` 返回 HTTP 200；`/v1/provider` 返回 `reachable=true`，当前模式为 `local-memory` / `Disconnected`。 |
| 鉴权边界 | 已确认 | 未带会话访问 `/v1/shell/state` 与 `/v1/admin/summary` 均返回 HTTP 401，公网接口仍保持 fail-closed。 |
| 部署版本 | 存在漂移 | 公网 `/v1/version` 返回 404；`/release-manifest.json` 虽为 200 但 `Content-Type` 为 `text/html`，属于 SPA fallback，不是真实 manifest；`creative.html` / `admin-ds.html` 静态资源最后修改时间仍为 2026-08-02。 |
| 结论 | 已登记 | 当前本地/ GitHub 主线的 H5 与 admin-ds 会话失效修复尚未由公网页面证明已部署；本次只读核验未 SSH、未重启、未改生产。 |

## 2026-08-13 公网版本追溯 smoke 收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 旧缺口 | 已修复 | `smoke-public-ingress.sh` 原先只验证页面、health、provider 和 401，无法发现 `/v1/version` 缺失或 `release-manifest.json` 被 SPA fallback 吞掉。 |
| 新合同 | 已实现 | 默认页面标记与当前 `index.html` / `creative.html` 对齐；新增运行时 `git_sha`、manifest `git_sha`、JSON `Content-Type` 和两者一致性校验；`EXPECT_RELEASE_GIT_SHA` 可锁定指定发布 commit。 |
| readiness | 已接入 | `production-readiness.sh CHECK_PUBLIC=1` 同步检查版本端点与 manifest，拒绝 404、HTML fallback、SHA 不一致和错误 release pin。 |
| 验证 | 已通过 | 本地真实 HTTP fixture 全链路通过；现网只读运行 smoke 在 `/v1/version` 404 处按预期失败，证明旧部署会被捕获；未修改生产。 |

## 2026-08-13 Linux 发布制品可审计收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 发布 commit | 已确认 | GitHub `main` 的可发布代码 commit 为 `d0e80f54539241381f10f3e256ec065b52083f77`；对应 release workflow run `31632458159` 全部通过。 |
| 架构制品 | 已生成 | 已生成 `x86_64-unknown-linux-gnu` 与 `aarch64-unknown-linux-gnu` 两份 source/web/Gateway 发布包，产物仅保存到外盘临时目录，未部署生产。 |
| 可移动校验 | 已通过 | 两份制品目录中的 `SHA256SUMS` 均为相对路径；在下载目录直接执行 `sha256sum -c SHA256SUMS`，三份 tarball 全部通过；`release-manifest.json.git_sha` 与发布 commit 完全一致。 |
| 架构与脱敏 | 已通过 | `file` 确认 Gateway 分别为 x86-64 与 ARM aarch64 ELF；source/web archive 未发现 `.env`、私钥/密钥文件、开发临时目录或高置信度 secret literal。 |
| 生产边界 | 未变更 | 本轮只生成、下载和验收 release artifact，未 SSH、未重启、未切换公网服务；公网旧部署仍会在 `/v1/version` 追溯检查处失败。 |

## 2026-08-13 H5 会话失效真实浏览器回归

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 覆盖缺口 | 已确认 | H5 `/v1/shell/state` 401 之前已有 auth/controller 与 creative runtime 单元回归，但真实双浏览器只覆盖正常双居民消息链路和 admin-ds 过期会话。 |
| 浏览器回归 | 已实现 | 增加独立 H5 browser context，注入脱敏过期 session，延迟 shell state 401，验证 token/身份清理、失效提示和登录 surface；H5 当前行为是直接打开登录 overlay，admin-ds 仍验证 HUD 点击入口。 |
| 隔离纪律 | 已实现 | H5 过期 fixture 与 index/creative 双居民页分离，避免失效状态污染真实发送、编辑、撤回和失败重试验收。 |
| 防回归 | 已通过 | `node scripts/smoke-web-dual-browser.mjs` 与完整 release gate 退出码 0；Web **1419/1419**、layout、realness、Gateway 323、TUI/CLI、双 HTTP、真实双浏览器（含 H5/admin-ds 401）和 provider federation smoke 全绿。 |
| 生产状态 | 未变更 | 仅本地 smoke、代码和文档验证，未 SSH、未部署生产。 |

## 2026-08-13 admin-ds 会话失效真实浏览器回归

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 覆盖缺口 | 已确认 | 之前 admin-ds 401/403 只有静态/单元回归，完整双浏览器 smoke 只覆盖 index/creative，未验证 classic `admin-ds.js` 与 deferred standalone auth 的真实加载顺序。 |
| 浏览器回归 | 已实现 | 双浏览器 smoke 增加独立 browser context 的 admin-ds 页面：注入脱敏 fixture session，延迟 `/v1/admin/summary` 的 401，验证 token/身份清理、登录 HUD 恢复和登录 overlay 可重新打开。 |
| 隔离纪律 | 已实现 | admin-ds 使用独立 origin storage context，避免过期会话 fixture 污染两个用户页的双端 IM 验收。 |
| 防回归 | 已通过 | `node scripts/smoke-web-dual-browser.mjs` 通过；原有双居民发送、编辑、撤回、失败重试继续通过；完整 release gate 退出码 0，含 Gateway 323、TUI/CLI、双 HTTP、真实双浏览器（含 admin-ds 401）和 provider federation smoke。 |
| 生产状态 | 未变更 | 仅本地 smoke、代码和文档验证，未 SSH、未部署生产。 |

## 2026-08-13 admin-ds Gateway 会话失效闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `admin-ds.js` 的 Gateway 读写 helper 原先只返回 HTTP 失败，不通知共享认证控制器；过期 Bearer 会让后台继续保留旧身份和登录 HUD。 |
| 认证接线 | 已实现 | `admin-ds.js` 对 401/403 统一通知 `shell-auth-standalone.js`；经典脚本早于 deferred module 执行时先暂存失效状态，模块初始化后立即消费。 |
| 失效语义 | 已实现 | 共享控制器清除 token/challenge，身份降为“访客”，HUD 回到“登录”，并保留“登录已失效，请重新登录”提示；新 OTP/新 token 自动清除失效标记。 |
| 防回归 | 已通过 | 定向 auth/admin-ds/H5 回归 130/130；Web **1419/1419**、layout、realness 通过；完整 release gate 退出码 0，含 Gateway 323、TUI/CLI、双 HTTP、真实双浏览器和 provider federation smoke。 |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署生产。 |

## 2026-08-13 H5 Gateway 会话失效闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | H5 `GET /v1/shell/state` 收到 401/403 时原先只清空 shell 投影，没有清除本地 Bearer session；初始化状态更新还可能把“登录已失效”覆盖成“空闲”，SSE/轮询因此持续携带过期会话 |
| 会话处理 | 已实现 | shell state、POST 和导出共用 Gateway 鉴权失败边界；shell state 401/403 现在清除 token、标记失效并保留重新登录提示；新 OTP/新 token 会清除失效标记 |
| 防回归 | 已通过 | Web **1418/1418**、layout、realness；新增 shell state 401 运行时回归和 auth controller 失效状态回归 |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署生产 |

## 2026-08-13 H5 合法空 Gateway 投影保持在线

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | Gateway `ShellState` 即使经过居民可见性过滤后没有任何会话，仍会返回带 `state_version` 和空数组的合法完整投影；H5 原先沿用本地 fixture 的“必须有非空房间”判定，会把新居民/无权限居民误报为 offline |
| 载荷合同 | 已实现 | 新增 Gateway 专用完整投影校验：要求非空 `state_version` 且至少存在 `rooms`、`conversation_shell.conversations` 或 `scene_render.scenes` 数组；空数组是合法状态，不回退 sample/cache |
| 状态标准化 | 已实现 | 共享 `normalizeShellStateForState` 保留合法空 Gateway 投影，H5 展示空会话态但维持 `gatewayConnection=online`；不完整/非对象投影继续 fail-closed |
| 防回归 | 已通过 | Web 1416/1416、layout、realness；完整 `scripts/smoke-release-gate.sh` 退出码 0，含 Gateway 323、真实双浏览器、双 HTTP、TUI/CLI 和 provider federation smoke |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署 |

## 2026-08-13 H5 Gateway shell state fail-closed

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 配置 Gateway 的 H5 在 shell state HTTP/网络失败时可能保留生成态或 IndexedDB 旧投影，provider 成功还可能把连接 badge 标成 online |
| 状态投影 | 已实现 | Gateway 配置下读取失败、空/畸形 payload 统一清空房间/消息投影、取消活动房间和 state version，不回退到 sample/cache；有效 Gateway payload 恢复后重新标记可用 |
| 连接状态 | 已实现 | 独立 `gatewayShellStateAvailable` 纳入连接状态聚合，provider 可达不再掩盖 IM shell 不可用 |
| 本地预览 | 保持 | 纯 `file:` 生成 fixture 继续走本地预览，不把 bootstrap 中的 loopback 开发地址误当正式 Gateway |
| 防回归 | 已通过 | Web 1413/1413；完整 `scripts/smoke-release-gate.sh` 退出码 0，含 Gateway 323、真实双浏览器、HTTP 双端和 provider federation smoke |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署 |

## 2026-08-13 P5 mirror source URL 安全合同

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | enabled world mirror 会被 federation read 主动抓取，但原先只做字符串归一化，可写入任意 HTTP |
| URL 合同 | 已实现 | enabled mirror 复用 provider URL 校验：远端仅 HTTPS；dev/test 仅允许 loopback HTTP；disabled mirror 保留配置兼容 |
| fail-closed | 已实现 | provider-config.json 载入 enabled 非法 mirror 时 Gateway 启动失败；新增/更新 mirror 在启用前拒绝非法 URL |
| 防回归 | 已实现 | Gateway 323/323，覆盖 enabled/disabled mirror 与 provider URL 合同复用 |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署 |

## 2026-08-13 P5 provider URL 安全合同

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `/v1/provider/connect` 原先直接接受 URL，未复用 readiness 的 HTTPS/loopback 约束；持久化 `provider-config.json` 也没有启动时安全校验 |
| URL 合同 | 已实现 | 远端只允许 HTTPS；dev/test 仅允许 `localhost`、`127.0.0.1`、`[::1]` 的 HTTP；拒绝凭据、空 host、非法端口和未知 scheme |
| fail-closed | 已实现 | provider connect、CLI/env 初始化和持久化配置载入统一走同一校验；非法持久化配置使 Gateway 启动失败，不静默连接 |
| 防回归 | 已实现 | 新增三组 URL/模式/持久化配置测试；`cargo test -p lobster-waku-gateway --quiet` 322/322 |
| 范围边界 | 保持 | 这只是 native transport 接入口的安全合同，不等于 native Waku；标准 MLS/Waku 组件仍需用户批准开源调研后选型 |
| 生产状态 | 未变更 | 仅本地代码、测试和文档验证，未 SSH、未部署 |

## 2026-08-13 P1 个人房间场景权限收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `home:<resident>` 已由 `personal_room_owner()` 明确定义，但 shell 场景写入只校验参与者；admin 私宅分支原先仅覆盖 `dm:*`，且没有区分房主 |
| 权限修复 | 已实现 | 个人房间 shell/admin 场景写入均要求房主；普通双人 `dm:*` 保持原有参与者可编辑行为；世界管家既有全局覆盖保持不变 |
| 防回归 | 已实现 | 新增 shell 与 admin 两组个人房间房主/非房主回归测试；`cargo test -p lobster-waku-gateway --quiet` 319/319 |
| 完整验证 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0，含 workspace、Clippy、CLI/TUI、Web 1412、双浏览器、provider federation、发布脚本与 panic scan |
| 生产状态 | 未变更 | 本阶段只修改本地代码、测试和文档，未 SSH、未部署；GitHub 同步待本地提交完成后进行 |
| 后续边界 | 保持 | P5 native Waku/标准 MLS 仍需用户批准开源调研后再选型；当前不引入新依赖、不把现有 federation/AES-GCM 骨架包装成正式实现 |

## 2026-08-12 P5 secure-session 密钥暴露收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| API 投影 | 已实现 | `POST /v1/direct/open` 改为只返回 `MlsGroupView` 元数据，响应不再包含 `group_key` |
| at-rest 存储 | 已实现 | `secure-sessions.json` 使用 `LOBSTER_SECURE_SESSION_MASTER_KEY` 派生密钥并以 AES-256-GCM 封装；旧明文快照仅一次性启动迁移并原子替换 |
| 生产门禁 | 已实现 | readiness 要求 master key 至少 32 字符且不打印值；未配置时生产持久化/读取 secure session 会失败关闭 |
| 验证 | 已通过 | `crypto-mls` 24/24、Gateway 316/316、完整 release gate、GitHub CI 和 Atlas 均已收口 |
| 边界 | 保持 | 这只保护当前 skeleton 的 API/磁盘暴露面；`crypto-mls` 仍非标准 MLS/E2EE，native Waku/MLS 选型仍待用户批准开源调研 |

## 2026-08-12 P5 federation 鉴权前置闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 真实边界 | 已厘清 | 当前跨节点能力是 HTTP gateway federation + local-memory backend，不是 native Waku；`crypto-mls` 仍是自研 AES-GCM 会话骨架，不是标准 MLS/E2EE，文档禁止混称 |
| 入站门禁 | 已实现 | 生产模式 `POST /v1/waku` 的 connect/subscribe/publish/recover/poll 全部要求专用 `LOBSTER_GATEWAY_FEDERATION_TOKEN`；缺失或错误 token 返回 401，runtime 仅保留域分离哈希 |
| 出站凭据 | 已实现 | 下游通过 `LOBSTER_WAKU_UPSTREAM_TOKEN` 在 Authorization header 携带凭据；TUI 直连协议面使用 `LOBSTER_WAKU_GATEWAY_TOKEN`；raw token 不写 provider config，客户端 Debug 固定脱敏 |
| 生产 readiness | 已收口 | 配置远程 `LOBSTER_WAKU_UPSTREAM_URL` 时必须同时配置 token，远程 URL 仅允许 HTTPS，loopback HTTP 仅供同机 sidecar/测试 |
| 验证 | 已通过 | 红测确认旧路由无门禁；实现后 production-mode 401/合法 token、client header+Debug 脱敏、transport 10/10、Gateway 315/315、TUI token 合同与真实双 gateway token federation smoke 全绿 |
| 下一步 | 待用户选择 | Atlas reuse gate 要求先确认是否允许限时开源调研，再决定 native Waku provider 与标准 MLS 库；未获确认前不引入依赖、不把骨架包装成正式实现 |

## 2026-08-12 S2 可追溯生产恢复准备

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| Linux CI | 已恢复 | 新 manifest 硬门禁暴露 Linux install-layout fixture 仍按旧合同安装；已修复并由 GitHub CI run 31609808497 全绿验证 |
| Release 制品 | 已生成 | GitHub release run 31610144716 的 verify、x86_64 Linux、aarch64 Linux 三个 job 全绿；制品绑定 `ee5039b975257dd5f0a245a266a152d1566092b8` |
| 公网基线 | 旧版健康 | `chat.ajw.cn` health/provider 与首页、creative、scene-editor、admin-ds 均 200；`/v1/version` 404，`/release-manifest.json` 仍 SPA fallback，证明尚未部署新追溯版本 |
| 执行边界 | 等待授权 | Atlas `host.aws-ec2-beijing` 明确 `agent_execution_allowed=false`；未 SSH、未修改生产。获得用户明确授权后再做备份、部署、版本/manifest、公网主链和回滚验收 |

## 2026-08-12 S1 发布可追溯性

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 运行版本 | 已实现 | Gateway 新增 `GET /v1/version`，返回 schema、Cargo 包版本和编译时完整 Git SHA；CI 通过 `LOBSTER_BUILD_GIT_SHA=${{ github.sha }}` 固化构建来源 |
| 发布清单 | 已实现 | `package-release.sh` 生成 `release-manifest.json`，记录 Git SHA、构建时间、target 以及 source/web/gateway 各制品文件名和 SHA-256 |
| 防误发 | 已实现 | 本地脏工作区默认拒绝打包；仅显式 `ALLOW_DIRTY_RELEASE=1` 可做本地验证。安装脚本在替换文件前校验 manifest Git SHA 和制品哈希 |
| 安装后关联 | 已实现 | 安装脚本保留 manifest，并检查本机 `/v1/version` 与公网 `/release-manifest.json`；部署文档已补齐查询方法 |
| 验证 | 全绿 | 脏工作区拒绝行为通过；显式本地打包后三类制品哈希复算一致；完整 release gate 通过（含 Rust、Web 1412、双浏览器、TUI、provider federation、安装/发布脚本与 panic 扫描） |
| 生产状态 | 未变更 | 本阶段只完成代码、CI 和本地制品验证，尚未部署；下一步需用已同步 GitHub 的确定 commit 构建制品，再执行生产备份、安装、版本/manifest/公网功能核验和回滚演练 |

## 2026-08-12 S0 源控救援

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 8 月 2 日 WIP | 已提交 | 场景交互/私宅空态、admin-ds 热点去重、备份自动化已按功能拆分提交 |
| 生成态污染 | 已修复 | TUI smoke 通过 `LOBSTER_WEB_GENERATED_DIR` 写临时目录；完整门禁后 tracked generated fixtures 不再变化 |
| federation 门禁 | 已去抖 | 先等待上游 health 再启动下游；单项连续 5/5、完整 release gate 通过 |
| 脱敏 | 已检查 | 新增生产记录中的真实测试邮箱和居民标识已移除；未发现 Token、验证码、API key 或私钥值 |
| 生产状态 | 未变更 | 本阶段只做本地源控收口与验证，没有重新部署；线上仍需在 S1 通过 commit/artifact manifest 建立精确追溯 |

## 2026-08-02 完整 release gate 复验（累计改动整体收口）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 完整门禁 | 通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0：Rust Gateway、TUI、CLI、Web 1412、双浏览器 smoke、provider federation、install/layout/public-ingress/package/release-workflow/production-readiness 各 unit、Rust production panic/unwrap scan 全过——今日五批改动（编辑器 UX/触控/私宅空态/cqh/去重）+ 备份脚本在完整门禁下整体可信 |
| 生产状态 | 稳定 | 部署后公网四页（index/creative/scene-editor/admin-ds）200、health 200；备份 timer 已启用待次日 03:40 首次触发 |
| 待用户动作 | 汇总 | 1) git commit（20+ 文件含已上线代码与文档）；2) DMARC TXT（可选加固）；3) admin-ds 去重版随下次部署上线 |

## 2026-08-02 备份脚本化与定期化（systemd timer 每日 03:30）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 备份脚本 | 已上线 | `scripts/backup-state.sh`（装到 `/opt/lobster-chat/scripts/`）固化 DEPLOYMENT §8：stop-tar-start + tarball 非空/含 timelines/可解压三重校验 + 磁盘水位检查 + trap 保证任何失败都拉起 Gateway + KEEP=14 轮转 |
| 定期化 | 已启用 | `scripts/systemd/lobster-chat-backup.{service,timer}` 装到 /etc/systemd/system 并 enable；每日 03:30 UTC ±15min（`Persistent=true` 补跑漏触发）；`systemctl list-timers` 确认下次 2026-08-03 03:40 |
| 手动验证 | 通过 | `systemctl start lobster-chat-backup.service` 全程 <1s：停机→归档→拉起→校验通过（archive 8.0K）；gateway active、health 200；DEPLOYMENT.md §8 已同步脚本/timer 用法，手动命令降级为兜底 |
| 范围说明 | 明确 | 仅状态目录 `/var/lib/lobster-chat`；`/etc/lobster-chat/*.env` 含密钥不纳入常规备份（重建走 install-server.sh + 密钥保管流程）；web 目录回滚锚点由部署时 web-backup-* 承担 |

## 2026-08-02 admin-ds 热点编辑器去重（两套表单合一）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 去重 | 已完成 | 房间详情内联版与场景模块页的热点表单合并为共享助手 `createHotspotListEditor(existingHotspots, {layout:'blocks'\|'flex', onRowsRendered})`（行渲染/删除/添加/收集），外加 `newHotspotDefaults` / `buildHotspotLayerPayload` / `buildImageLayerPayload`；两调用点只保留各自容器结构、layer_id 前缀（admin-custom-/admin-scene-）和保存后动作；admin-ds.js 净减约 60 行 |
| 顺手清除 | 已完成 | 场景模块页保存逻辑里的死代码 `querySelectorAll('[data-no-clear]')` 无用变量；两版重复的坐标字段表合一为 `HOTSPOT_COORD_FIELDS` |
| 行为保持 | 已核对 | 三态清除语义（空热点/空图像层显式 null）、标题计数同步、保存后各自刷新路径不变；两条静态断言按新结构重写但保护意图不变（成对昼夜、null 清除、计数回调） |
| 防回归 | 全绿 | unit 1412/1412、layout、realness 通过；`admin-ds.js` 升版 `?v=20260802-hotspot-dedup` |
| 部署状态 | 未部署 | 纯重构无视觉变化，admin-ds 为后台页，随下一功能批次一起上生产即可 |
| 后续候选 | 待做 | empty-note 视觉统一（低价值，可降级）；P3 app.js 剩余候选 3/4/5（蓝图标注边际递减，不建议做） |

## 2026-08-02 P1 polish 四批次生产部署（web 静态）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 部署内容 | 已上线 | 场景编辑器入口+触控/键盘、移动端触控目标、私宅空态卡片、cqh 高度基准四批次；版本号 `20260802-scene-canvas-cqh` / `20260802-touch-targets` / `20260802-locked-card` / `20260802-scene-editor-entry` |
| 打包 | 已校验 | 复刻 `package-release.sh` 排除规则本地打包（56M），`COPYFILE_DISABLE=1`；sha256 双端一致；上传前本地解包逐文件 size 比对（外盘曾瞬读 0 字节虚警，实为挂载抖动，文件本身完好） |
| 安装 | 原子交换 | 先 `/srv/backups/web-backup-20260802-082217.tar.gz` 全量备份（56M，回滚锚点）；新包解到临时目录 → 零字节/关键文件检查 → `web-new` → `mv` 原子交换（摒弃 rm -rf 先行）；期间 SSH 多次瞬断，改 nohup  detached 执行 + 日志文件，防断线半途 |
| 公网验证 | 通过 | `index.html` 公网已带新版本号；新 pixel-map.css 200（CF MISS 新 URL 自然失效旧缓存，无需 Purge）；`shell-private-room-locked-card.js` 200；assets 0 字节 PNG=0；health 200 |
| 教训 | 已记录 | macOS 本地打包：`COPYFILE_DISABLE=1` 只挡 `._*` AppleDouble，pax xattr 关键字仍会写入但 GNU tar 只警告不影响内容；AWS 北京 SSH 在大传输后易瞬断，长操作一律 nohup + 日志；部署前零字节检查需排除已知残留 `styles.world-square.css.tmp`（0 字节，CI 打包同样会带上，无害） |

## 2026-08-02 scene-canvas 高度基准修复（100vh → 容器查询 cqh）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 运行时热点层宽度按 `100vh × 16/9` 推算（2026-05-18 layout hardening 遗留），stage 实际高度被顶栏/移动端 Tab Bar/safe-area 占去后，热点坐标盒与 `contain` 背景渲染盒错位；基准规则 `aspect-ratio + max-height` 冲突时比例失效同样致错位 |
| 修复 | 已完成 | `.creative-stage` 加 `container-type: size`，热点层改 `width: min(100%, calc(100cqh × var(--creative-scene-aspect)))` + `margin: auto`：画布恒等于 min(stage内容宽, stage内容高×16/9) 且自保持 16:9，与背景 contain 盒严格对齐；桌面视觉不变 |
| 验收 | 全绿 | `verify-scene-layout.mjs` 新增 2 个 case（390×844 移动竖屏、1560×873 桌面）断言热点画布宽与 16:9 比例；断言用 clientWidth/clientHeight（cq 单位相对 content-box，比 getBoundingClientRect 少 2px 边框）；unit 1412/1412、realness 通过 |
| 缓存纪律 | 已执行 | pixel-map.css 引用再升版 `?v=20260802-scene-canvas-cqh`（3 页 + 3 处测试断言同步） |
| 后续候选 | 待做 | admin-ds 两套热点表单去重（中）；empty-note 视觉统一（中）；public-square stage 如需同款 cqh 对齐可复用本模式 |

## 2026-08-02 未授权私宅 stage 空态卡片

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 缺口 | 已确认 | 点击未授权私宅此前只有 governance 状态条一行字，stage/timeline 区域无任何视觉反馈 |
| 空态卡片 | 已完成 | 新模块 `shell-private-room-locked-card.js` 纯模型：把 `residentPrivateRoomAccessPromptModel` 五态（未登录/需好友/已申请/待接受/好友待同步）升级为 timeline 区居中卡片，复用 `createTimelineEmptyStateNode` 结构（title/copy/action），copy 一律用 Gateway 投影原文不伪造；app.js `enterResidentRoom` 的 accessPrompt 分支挂载，下一次 focus/refresh 经 `clearChildren(timelineEl)` 自然覆盖 |
| 卡片样式 | 已完成 | `styles.user-shell.css` 自洽卡片样式（creative 页不加载 styles.chat.css）：dark-on-dark `rgba(22,16,12,.88)` + `#3a2f28`，行动提示按状态分色（actionable 冷青/locked 暖红/pending 金）；引用升版 `?v=20260802-locked-card`（3 页） |
| 测试基建 | 已补齐 | 新 shell 模块必须登记进 `test/fake-dom.mjs` 模块清单，否则 fake-DOM import rewrite 在临时目录找不到模块、67 个用例级联失败——本次已登记并验证 |
| 防回归 | 全绿 | 新增 `shell-private-room-locked-card.test.mjs` 5 用例（结构/tone/五态文案/不伪造文案/app.js 接线/CSS 深色断言）；Web 1412/1412、layout、realness 通过 |
| 后续候选 | 待做 | admin-ds 两套热点表单去重（中）；scene-canvas 高度基准 100vh→容器（中）；empty-note 视觉统一（中） |

## 2026-08-02 移动端触控目标批次（≥34px 约定收口）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 口径确认 | 已对齐 | 项目移动端触控底线是 **≥34px**（verify-frontend-realness 钉住关系按钮 34px），不引入 Apple 44px 新标准；本批只抬基准以下元素，34px 及以上的（composer-symbol-menu 34、hud-login-toggle 34、wechat-rail-toggle 36/40、chat-detail-card-action 38）不动 |
| pixel-map 3 处 | 已完成 | 文件末尾新增 ≤820px 块：`.message-action` 26→34、`.composer-symbol-tab` 28→34、`.public-square-mention-chip` 30→34（!important 对齐基准规则） |
| world-square / creative | 已完成 | `.world-square-actions a/.hud-login-toggle/.hud-pill` 28→34（≤820px）；`.resident-login-close` 32→36（≤820px） |
| 缓存纪律 | 已执行 | 三个改动 CSS 引用统一升版 `?v=20260802-touch-targets`（pixel-map×3 页、world-square×1、creative×5 页）；shell-pages-static 版本断言同步更新（含漏网的 world-square 一条） |
| 防回归 | 全绿 | 新增 "mobile touch targets meet the 34px floor" 静态断言；Web 1407/1407、layout、realness 通过 |
| 后续候选 | 待做 | admin-ds 两套热点表单去重（中）；运行时 scene-canvas 高度基准 100vh→容器（中）；未授权私宅 stage 空态卡片（中）；empty-note 视觉统一（中） |

## 2026-08-02 P1 场景编辑器 UX polish 第一批（可视化编辑器入口 + 触控/键盘微调）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| admin-ds 可视化编辑器入口 | 已完成 | `renderSceneEditor` 顶部新增「打开可视化编辑器」入口卡，`admin-ds.js` 拼装 `./scene-editor.html?gateway=&room=&token=&identity=`（URL 合同与 `app.js sceneEditorUrlForCurrentState()` 一致），新标签页 `rel=noopener`；此前 scene-editor.html 在后台无任何入口 |
| scene-editor 触控热区 | 已完成 | resize 手柄视觉保持 10px，`::before inset:-7px` 把可点区域扩到 24px；删除钮 18px→28px（top 同步 -26→-32） |
| scene-editor 方向键微调 | 已完成 | 选中热点后方向键 0.5% 步进（50 permyriad，Shift=250 大步），输入框聚焦不拦截；连续按住经 `lastNudgeAt` 600ms 合并只推一次 undo 快照；边界 clamp 复用 drag 语义 |
| 缓存纪律 | 已执行 | admin-ds.html 的 `admin-ds.js` 引用首次补 `?v=20260802-scene-editor-entry`；`admin-ds-static.test.mjs` 断言同步放宽为允许 `?v=`（此前无版本参数，CF 边缘 max-age=14400 下改动拿不到） |
| 防回归 | 全绿 | 新增静态断言 2 条（admin-ds 入口 URL 合同、scene-editor 触控/方向键）；Web 1406/1406、layout、realness 通过 |
| 后续候选 | 待做 | admin-ds 两套热点表单去重（中）；运行时 scene-canvas 高度基准 100vh→容器（中，需先补 verify-scene-layout 移动端 case）；其余移动端触控目标（.message-action 26px 等 9 处）；未授权私宅 stage 空态卡片 |

## 2026-08-01 Resend 域名收口 + 双居民生产验收 + admin 恢复演练（当日上午阻塞项全部解除）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| Resend 域名验证 | 已 Verified | 用户在 Resend dashboard 添加 chat.ajw.cn（Tokyo ap-northeast-1），Auto configure 经 Cloudflare OAuth 自动写入 DNS；DKIM `resend._domainkey.chat`、SPF `send.chat`、MX `send.chat→feedback-smtp.ap-northeast-1.amazonses.com` 三条权威记录当日生效并 Verified；DMARC 可选未配，Enable Receiving 未开（OTP 仅发不收，不需要） |
| 正式发件人 | 已切换 | `LOBSTER_MAILER_FROM` 改为 `Lobster Chat <no-reply@chat.ajw.cn>` 并重启 mailer；`gateway.env` 的 `LOBSTER_EMAIL_OTP_FROM` 此前已预设该值（07-28 曾因域名未验证 403），域名 Verified 后自然生效；两 env 变更前均留 `.bak-<ts>` 备份 |
| 投递实测 | 通过 | 真实 OTP 经公网 gateway→mailer→Resend 投递 gmail/qq 两邮箱均成功（mailer 仅记录失败日志，全程零 fail）；challenge 响应无 dev_code、邮箱脱敏正常 |
| 平台管理员能力 | 已补齐 | 发现 `ban:resident` 等能力由 `LOBSTER_SUPER_ADMINS` 或权限组授予，生产此前未配置致无人可执行管理动作；已在 gateway.env 加 `LOBSTER_SUPER_ADMINS="rsaga"`（与 governance-state `world_stewards=["rsaga"]` 对齐）并重启 gateway，停机秒级 |
| admin 恢复动作演练 | 通过 | 按 ACCOUNT_APPEAL_RUNBOOK：只读调查（脱敏邮箱/注册态/sanctions）→ ban 测试居民（is_banned=True）→ unban（lifted_count=1，is_banned=False）→ audit-log 留痕 `admin:ban_resident`/`admin:unban_resident`（actor 使用脱敏管理员标识，含 reason 与时间戳） |
| 第二测试居民 | 已注册 | 第二测试邮箱经真实 OTP 注册，resident_id 使用脱敏测试标识，registration_state=active，shell state 可读（4 个可见房间） |
| 双居民私聊验收 | 8/8 通过 | 脱敏测试居民 A/B：direct/open、A→B 发送与对端可见、B 回复与对端可见、edit（edited）、recall（recalled）、终态双侧确认；全部走生产 API 无伪造 |
| 现场清理 | 已完成 | 主机 /tmp 会话 token 文件、验收脚本已删除；env 备份保留在 /etc/lobster-chat/*.bak-* |
| 待办（不阻塞） | 可选 | 补 DMARC TXT（`_dmarc.chat.ajw.cn`, `v=DMARC1; p=none;` 起步）；admin-ds 浏览器端视觉复核本轮 API 级操作 |

## 2026-08-01 生产运维收口演练（备份/恢复/健康/邮件域名核查）

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 生产健康验证 | 通过 | `production-readiness.sh CHECK_PUBLIC=1 BASE_URL=https://chat.ajw.cn` 在生产主机上以 sudo 执行退出码 0（脚本需读 `/etc/lobster-chat/gateway.env`，普通用户不可读）；公网 `/health`、本机 gateway `:8787/health`、mailer `:8791/health` 均 200，两服务 active |
| 备份演练 | 通过 | 按 DEPLOYMENT §8：stop gateway → `tar -C /var/lib -czf /srv/backups/lobster-chat-state-20260801-053020.tar.gz lobster-chat` → start，停机秒级；重启后 health 200。状态目录现仅 64K（4 条 timeline + 5 个 JSON），生产尚新属正常 |
| 恢复演练 | 通过 | 备份解至 home 临时目录（避开 tmpfs /tmp），9 个文件逐一 sha256 与现网一致、timeline 计数一致；临时目录已清理，现网状态未触碰 |
| 回滚锚点 | 在位 | `/opt/lobster-chat/bin/lobster-waku-gateway.bak-7b0218d` 存在；`install-server.sh` 重装旧包路径未实操（无故障无需回滚） |
| admin-ds 只读面 | 部分通过 | admin-ds.html 公网 200；`/v1/admin/summary`、`/v1/admin/residents` 无 Bearer 均 401；`audit-log.json` 为 NDJSON，auth:login/logout 留痕可追踪；Gateway journal 自 07-28 以来 0 条 bearer/OTP/密码敏感泄露 |
| admin 恢复动作演练 | 阻塞（待用户配合） | 管理端点鉴权=居民 Bearer session，session 只存 token 哈希无法从状态复原；需管理员重新 OTP 登录（验证码进入已绑定管理员邮箱，地址不写入公开仓库）后才能演练 unban/unsanction 恢复动作 |
| Resend 域名验证 | 未验证（确认） | mailer 日志实证：07-28 03:10 `403 The chat.ajw.cn domain is not verified`；当前 API key 为 sending-only（`/domains` 返回 restricted_api_key），无法 API 查域名状态。后果：当时仅可向 Resend 账户 owner 邮箱发信（地址不写入公开仓库），发件人维持 `onboarding@resend.dev` |
| 后续待办 | 待用户外部动作 | 1) 在 Resend dashboard 验证 chat.ajw.cn（DNS SPF/DKIM）；2) 验证后切换 `LOBSTER_MAILER_FROM` 正式发件人；3) 注册第二测试居民补双居民私聊生产验收；4) 配合一次管理员 OTP 登录完成恢复动作演练 |

## 2026-07-31 三页框架宽度与边框统一

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 宽度不一致 | 已修复 | 主城原 min(1440px,100%) 居中、世界原 min(100vw,1640px) 宽屏偏窄,统一 100vw 与住宅一致;1920 实测三页 w=1920 left=0 |
| 边框色调不统一 | 已修复 | 2026-06-18"三页 HUD 暖金一致"与去蜡黄收口冲突:hud 金框、stage 金环、rail 金边、世界页金框/导航金底统一 #3a2f28 深色+active 冷青左条,金色仅留小文字;1920 实测三页 stage border 同为 rgb(58,47,40)/1px 无金环 |

## 2026-07-31 场景背景图全灭事故修复(macOS tar AppleDouble)

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | UI refresh 期间用 macOS bsdtar 手动打包 web 部署,带 xattr 的 PNG 在 GNU tar 解出 0 字节实体 + `._*` AppleDouble 残片,65 张图全空;期间 EC2 /tmp(tmpfs 457M)被打包文件塞满一次 |
| 修复 | 已上线 | `COPYFILE_DISABLE=1` 重新打包、清 `._*` 后重部署(assets 9.1M→55M);0 字节响应被 CF 边缘缓存(age/14400),Purge 后恢复 |
| 教训 | 已记录 | 手动打包必须 `COPYFILE_DISABLE=1`;优先用 CI GNU tar artifact;EC2 /tmp 是 tmpfs,大包走 home 目录;图片响应也会被边缘缓存,部署后抽查关键资产 size_download |

## 2026-07-30 H5 UI refresh 三期全量上线(P0/P1/P2)

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 策划与原型 | 用户已确认 | `docs/UI_REFRESH_PROPOSAL_20260728.md` + `docs/mockups/ui-refresh-20260728-prototype.html`;三层结构(住宅场景优先/主城对话优先/世界发现优先)确认后实施;回滚锚点 tag `ui-refresh-base-20260728` |
| P0 顶栏+气泡+动作收敛 | 已上线 | ≤720px 顶栏单行;气泡/composer 深色化——根因是 pixel-map.css 内"final contract" !important 规则钉死 cream,外部覆盖无效,已直接改写;编辑/撤回收敛进长按/右键动作面板(新模块 shell-message-action-sheet,3 单测);顶栏紧凑规则须放 pixel-map.css(creative 不加载 styles.scene.css) |
| P1 底 Tab+微信适配 | 已上线 | 三页底部 Tab Bar(≤820px,住宅/主城/世界/我的,composer 自动抬升);微信专项:overscroll-behavior:none 防下拉误刷新、100dvh、输入字号 ≥16px、safe-area |
| P2 桌面深色化 | 已上线 | rail active 金底改深底+冷青左条、居民/城市卡 cream 改深色、退出键中性描边、消息区限宽 720px 居中 |
| 验证 | 全绿 | Web 1404/1404(+3 新单测)、layout、realness、完整 release gate;生产三形态截图复核;meta 可读性(昵称金色小字/时间浅灰)单独修复 |
| Atlas / 生产事实 | 已核验 | Atlas 将 `deployment.lobster-chat-production` 标记为 active/verified；chat.ajw.cn 健康检查、13/13 双端 IM 验收与 UI refresh 视觉复核均有记录；本次同步不包含主机地址、邮箱或凭据值 |
| Git 同步 | 本次已完成 | 当前工作分支已与 origin 同名分支同 commit；本次仅提交脱敏进度记录，运行态 generated JSON 不纳入提交 |

## 2026-07-28 生产部署落地 chat.ajw.cn(AWS EC2 北京)

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| Git 收口 | 已完成 | 113 修改 + 54 未跟踪分 6 批提交;`refactor/appjs-techdebt-20260710` 推送 origin 并快进合并入 main(842f170→7b0218d);临时产物入 gitignore |
| CI | 已转绿 | lobster-chat-ci 双 job 首次全绿:修新版 stable clippy×5、rustfmt、rust-smoke 补 Node/npm ci 与 Playwright chromium、根 package.json 显式声明 playwright@1.59.1;release.yml verify 补 chromium |
| release 构建 | 已完成 | workflow_dispatch 构建 x86_64+aarch64 artifact,SHA256 校验一致 |
| 生产部署 | 已完成 | AWS 北京生产节点以 `install-server.sh` artifact 模式装 gateway、web、nginx；gateway.env 关闭开发开关并使用同机 mailer，`lobster-mailer` 已启用并健康 |
| 公网入口 | 已完成 | cloudflared tunnel(2d0e230a)新增 chat.ajw.cn→localhost:80;CNAME 已建;`smoke-public-ingress.sh` 与 `production-readiness.sh CHECK_PUBLIC=1` 全过 |
| 安全边界 | 已验证 | 无 Bearer 401、OTP 响应无 dev_code、CORS 单 origin、journal 无敏感日志;OTP 投递失败文案收口为通用 `email otp delivery failed`(细节只进服务端日志),mailer 未启用时认证失败关闭 |
| 生产发现的脚本坑 | 已修复 | smoke-public-ingress CORS 断言改 `grep -Fi`(HTTP/2 小写头);env 中含空格/尖括号值必须加引号(DEPLOYMENT 示例已改) |
| 待用户外部动作 | 已完成 | Resend 已注册并取得 API Key；`lobster-mailer` 已 enable(active,health 200)；当前仍使用测试发件地址，正式域名验证后再切换正式发件人并补齐 SPF/DKIM |
| 真实邮件 OTP | 已验收 | 2026-07-28 经公网向已绑定测试邮箱发起 OTP：`dev_code=null`、邮箱已脱敏、`delivery_mode=mailer-webhook`；verify/logout 与双端 IM 验收已由后续生产记录确认 |

## 2026-07-28 生产 H5 三处真修复 + 双端 IM 验收 13/13

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 真实邮箱注册链路 | 验收通过 | 测试邮箱收件→verify(测试居民,Active)→session 可读 shell state→logout→旧 token 立即 401 |
| pretext vendor 化 | 已修复上线 | pretext-stage.js 引用 ./node_modules/...,artifact 不含 node_modules,nginx 回退 index.html 致 app.js 模块图整体加载失败;改 vendor/pretext 随包分发,静态测试防回归 |
| https 同源回退 | 已修复上线 | hub 页无 ?gateway= 时原返回 null;https 下回退 location.origin;打包带入的 dev loopback bootstrap 地址在 https 下忽略 |
| 轮询鉴权 | 已修复上线 | loadGatewayState 未带 Bearer,/v1/shell/state 401 静默失败,界面永远停留静态 state.json;EventSource 401 自动回退轮询 |
| CDN 缓存 | 已修复上线 | CF 边缘 max-age=14400 持旧模块致部署不一致;nginx 对 js/css/html 发 no-cache(源站+install-server.sh 模板),Purge Everything 后恢复 |
| 双端 IM 验收 | **13/13 通过** | 双浏览器会话:登录态、双方向实时收发、失败气泡+重发无重复、搜索(带 token 200/无 token 401)、编辑/撤回 API 与界面反映、零 JS 错误;重启 Gateway 后消息可检索 |
| 待办 | 待做 | admin-ds 运维演练、备份/恢复演练；Resend 域名验证后第二测试居民注册并补私聊双居民验收 |

## 2026-07-27 H5 会话摘要表面 DOM 职责下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 前端减债 | 已完成 | 新增 `apps/lobster-web-shell/shell-room-digest-surfaces.js`，承接最近会话标题、上下文摘要、统计徽章和当前管家状态；`app.js` 只保留 renderer 创建、依赖注入与 `renderRoomDigest` 委托 |
| 遗留代码 | 已清理 | 删除 `app.js` 中会话摘要 DOM helper，`app.js` 由 7,542 行降至 7,460 行；Gateway 状态、pending echo、会话统计和页面文案契约保持不变 |
| 防回归 | 已通过 | 会话摘要模块 2/2；静态套件 166/166；Web `npm test` 1,399/1,399；layout、frontend realness、fake DOM import 与 `git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233，认证、居民主链、双 HTTP、双浏览器和 provider federation 均通过 |
| 外部验收边界 | 保持 | Atlas 当前仍没有 lobster-chat 生产环境声明；目标 Linux 主机、正式域名/TLS、真实邮件 OTP 与公网双端 IM 仍待，未部署、未提交 Git |

## 2026-07-27 H5 房间列表 DOM/分组表面职责下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 前端减债 | 已完成 | 新增 `apps/lobster-web-shell/shell-room-list-surfaces.js`，承接房间空态、工具栏摘要、头像/未读、标签、房间项与分组；`app.js` 只保留 `renderRooms` 编排代理和依赖注入 |
| 遗留代码 | 已清理 | 删除 `app.js` 中抽取后的 room list DOM helper，`app.js` 由 7,718 行降至 7,542 行；未改变 room rail 纯模型、Gateway 合同或过滤/分组排序规则 |
| 防回归 | 已通过 | 房间列表表面静态门禁与完整静态套件 166/166；Web `npm test` 1,397/1,397，layout、frontend realness、fake DOM import 与 `git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233、Web 1,397，认证、居民主链、双 HTTP、直接会话、双浏览器和 provider federation 均通过 |
| 外部验收边界 | 保持 | Atlas 当前仍没有 lobster-chat 生产环境声明；目标 Linux 主机、正式域名/TLS、真实邮件 OTP 与公网双端 IM 仍待，未部署、未提交 Git |

## 2026-07-27 H5 居民目录与紧凑居民列表 DOM/关系动作职责下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 前端减债 | 已完成 | 新增 `apps/lobster-web-shell/shell-resident-surfaces.js`，承接居民目录卡片、住宅页紧凑居民列表、头像/在线状态、私聊入口和好友关系动作；`app.js` 只保留页面编排与 Gateway 依赖注入 |
| 遗留代码 | 已清理 | 删除 `app.js` 中抽取后的居民 DOM/关系 action helper，`app.js` 由 7,939 行降至 7,718 行；没有改变 Gateway 状态真源、身份边界或关系路由 |
| 防回归 | 已通过 | 居民表面静态门禁与完整静态套件 166/166；Web `npm test` 1,397/1,397，layout、frontend realness、fake DOM import 与 `git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233、Web 1,397，认证、居民主链、双 HTTP、直接会话、双浏览器和 provider federation 均通过 |
| 外部验收边界 | 保持 | Atlas 当前仍没有 lobster-chat 生产环境声明；目标 Linux 主机、正式域名/TLS、真实邮件 OTP 与公网双端 IM 仍待，未部署、未提交 Git |

## 2026-07-27 H5 治理城市卡片与动作 DOM 渲染职责下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 前端减债 | 已完成 | 新增 `apps/lobster-web-shell/shell-governance-city-surfaces.js`，承接治理城市卡片、公共房间/居民列表、加入/开房/冻结/执事/联邦策略动作和离线空态；`app.js` 只保留 Gateway 投影编排与依赖注入 |
| 遗留代码 | 已清理 | 删除 `app.js` 中抽取后的治理城市 DOM helper，`app.js` 由 8,259 行降至 7,939 行；未改变 Gateway 合同或状态真源 |
| 防回归 | 已通过 | 治理静态门禁 166/166；Web `npm test` 1,397/1,397，layout、frontend realness、fake DOM import 与 `git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233、Web 1,397，认证、居民主链、双浏览器、直接会话和 provider federation 均通过 |
| 外部验收边界 | 保持 | Atlas 当前仍没有 lobster-chat 生产环境声明；目标 Linux 主机、正式域名/TLS、真实邮件 OTP 与公网双端 IM 仍待，未提交 Git |

## 2026-07-27 H5 治理/世界公共表面 DOM 渲染职责下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 前端减债 | 已完成 | 新增 `apps/lobster-web-shell/shell-world-surfaces.js`，统一承接世界目录、镜像源、世界广场和世界安全表面的 DOM 工厂；`app.js` 只保留注入依赖和渲染委托，Gateway 投影与页面行为不变 |
| 遗留代码 | 已清理 | 删除 `app.js` 中抽取后不再调用的世界安全 DOM helper，`app.js` 由 8,470 行降至 8,259 行 |
| 防回归 | 已通过 | 世界表面静态门禁 166/166；Web `npm test` 1,397/1,397，layout、frontend realness 与 `git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233、Web 1,397，双浏览器、认证、居民主链、直接会话和 provider federation 均通过 |
| 外部验收边界 | 保持 | Atlas 当前仍没有 lobster-chat 生产环境声明；目标 Linux 主机、正式域名/TLS、真实邮件 OTP 与公网双端 IM 仍待，未提交 Git |

## 2026-07-27 H5 场景编辑器 Gateway 读写身份契约收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 编辑器加载只依赖旧 `rooms` 路径，入口未透传当前居民身份，保存缺少身份时回退伪造 `user`；Gateway 实际要求 `resident_id` 与 Bearer 会话绑定，并以 `rooms/scene_render` 提供场景图层 |
| H5 修复 | 已完成 | scene-editor 入口携带当前居民身份；加载按居民请求并兼容 Gateway `rooms`、`scene_render`、旧合同；无登录居民身份时保存明确阻断并使用真实 actor，不再伪造身份；无图层/未找到房间时清除旧画面与热点状态 |
| 防回归 | 已通过 | 新增场景编辑器 Gateway 合同/身份静态测试；Web `npm test` 1399/1399，layout、frontend realness、Gateway 场景写入 3/3、`git diff --check` 通过 |
| 外部验收边界 | 保持 | 目标 Linux 主机、正式域名/TLS、真实邮件 OTP 和公网双端 IM 仍待；Atlas 当前没有 lobster-chat 生产环境声明；未提交 Git |

## 2026-07-27 H5 访客发送与合成 QA 身份边界收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 真实 production-mode Gateway 无 Bearer 时，带 `identity` query 的住宅页仍可能把 query 身份当成居民，输入框可编辑并把发送错误推迟到后端 `401`；query identity 不是认证凭据 |
| H5 修复 | 已完成 | Gateway 页面无 session 时忽略 query identity、按 `访客` 加载；composer 需要 Bearer session；只有 loopback 且显式 `qa=browser`/`qa=manual` 才允许合成 QA 身份 |
| 测试夹具 | 已完成 | Gateway 身份夹具补齐测试 session token；消息编辑/撤回和非用户壳 placeholder 断言同步当前 session/待同步合同 |
| 真实浏览器 | 已通过 | 390×844 `creative.html` + 无开发 bypass/无 token Gateway：访客 scope、OTP 登录提示、输入和发送均 disabled；loopback `qa=browser` 主城仍完成真实发送并收到 committed self bubble |
| 防回归 | 已通过 | focused 227/227；Web `npm test` 1399/1399，layout、frontend realness、`git diff --check` 通过 |
| 外部验收边界 | 保持 | 目标 Linux 主机、正式域名/TLS、真实邮件 OTP 和公网双端 IM 仍待；Atlas 当前没有 lobster-chat 生产环境声明；未提交 Git |

## 2026-07-27 admin-ds 场景编辑器热点计数投影收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 真实移动端浏览器中添加/删除热点后，编辑列表已更新但标题仍保留初始 `0 个` |
| 前端修复 | 已完成 | `admin-ds.js` 保存热点标题节点，并在 `renderHotspotRows()` 每次重绘时从 `existingHotspots.length` 更新计数 |
| 真实浏览器 | 已通过 | 390×844 admin-ds：添加热点显示 `1 个`，删除后恢复 `0 个` 并显示“暂无热点” |
| 防回归 | 已通过 | admin-ds 定向测试 81/81；Web `npm test` 1395/1395；layout/realness、`git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312、TUI 233、Web 1395、双浏览器、认证、居民主链、Provider federation 与发布门禁通过 |
| 外部验收边界 | 保持 | 目标 Linux 主机、正式域名/TLS、真实邮件 OTP 和公网双端 IM 仍待；Atlas 当前没有 lobster-chat 生产环境声明；未提交 Git |

## 2026-07-27 admin-ds 告警 badge 真源投影收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 后台日志 badge、待处理统计和顶部告警数在 Gateway 异步读取审计日志后仍可能保留静态 demo 计数（原为 7/3），与真实投影不一致 |
| 前端修复 | 已完成 | `admin-ds.js` 增加 `updateAlertCounts()`，消息和日志 badge、仪表盘待处理告警及顶部告警数统一从当前 `messages`/`logs` 计算；Gateway 空数组会明确投影为 0 并隐藏 badge |
| 防回归 | 已通过 | `admin-ds-static.test.mjs` + `admin-ds-runtime.test.mjs`：80/80；Web `npm test`：1394/1394；`git diff --check` 通过 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312/312、TUI 233/233、Web 1394/1394、双浏览器、认证、居民主链、Provider federation 与公网 ingress smoke 均通过 |
| 外部验收边界 | 保持 | 仍需目标 Linux 主机、正式域名/TLS、真实邮件 OTP 和公网双端 IM；Atlas 当前没有 lobster-chat 生产环境声明；未提交 Git |

## 2026-07-27 公网 ingress smoke 契约加固

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 验收脚本 | 已完成 | `scripts/smoke-public-ingress.sh` 默认首页标记同步到当前 `index.html`，并增加 `creative.html`、`admin-ds.html`、无 Bearer 的管理/退出接口 `401` 边界，以及可选 `EXPECT_CORS_ORIGIN` 校验 |
| 防回归 | 已完成 | `test_smoke_public_ingress_unit.py` 锁定上述合同；脚本语法、quick unit 和 `git diff --check` 通过 |
| 本地真实入口 | 已通过 | 临时真实 Gateway（开发鉴权关闭）+ 静态 Web 反向代理场景通过主页、H5、后台、GET/HEAD health、provider、401 和 CORS 检查 |
| 完整门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 312/312、TUI 233/233、Web 1393/1393 |
| 外部验收边界 | 保持 | 仍需目标 Linux 主机、正式域名/TLS、真实邮件 OTP 和公网双端 IM；Atlas 当前没有 lobster-chat 生产环境声明；未提交 Git |

## 2026-07-27 H5 与后台服务端 logout 主链收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | Gateway/CLI 已有服务端 revoke，但 H5/admin-ds 只有本地清理或没有可见退出入口，无法从页面证明旧 Bearer 已撤销 |
| H5 修复 | 已完成 | shared auth controller 增加 `/v1/auth/logout` Bearer POST；主 H5 与 admin-ds HUD 登录按钮在已登录态显示“退出登录” |
| 失败语义 | 已完成 | 服务端退出失败仍清理本地 token，但明确显示“网关退出待重试”，不伪报服务端已撤销；后台身份变化后重新加载 Gateway 投影 |
| 防回归 | 已通过 | shell-auth logout 8/8；Web `npm test` 1393/1393；Gateway logout/revoke 既有测试、Rust workspace、fmt、Clippy `-D warnings` 与完整 release gate 通过 |
| 外部验收边界 | 保持 | 目标 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 OTP 与 Bearer session token 熵源收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | OTP 与 Bearer session token 原先由消息序号、时间和 challenge/居民标识可预测地派生，不能作为生产认证凭据熵源 |
| Gateway 修复 | 已完成 | OTP、challenge ID 和 session token 改用操作系统 CSPRNG；OTP 拒绝 modulo-bias 尾部；随机源不可用时认证请求失败关闭 |
| 防回归 | 已通过 | Gateway 312/312、Clippy `-D warnings`、Rust fmt、`git diff --check` 和完整 release gate（含认证、公网双浏览器模拟、provider federation）通过 |
| 外部验收边界 | 保持 | 目标 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 H5 消息搜索认证与查看者可见性收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `/v1/shell/messages/search` 原先未要求 Bearer，也直接扫描全局活动会话，未参与者可读取私聊正文；H5 请求也未带当前居民身份/会话令牌 |
| Gateway 修复 | 已完成 | 搜索要求 `resident_id` 并绑定匹配 Bearer session；结果复用 H5 会话可见性，个人房间正文继续 owner-only；缺失/错配认证返回 400/401 |
| H5 修复 | 已完成 | `messageSearchRequestModel()` 携带 `resident_id`，控制器动态透传当前居民身份和 `Authorization: Bearer ...`；访客不发起搜索请求 |
| 防回归 | 已完成 | Gateway 覆盖未认证、参与者可见和非参与者空结果；Web 搜索/静态测试与 Rust fmt、`git diff --check` 通过 |
| 外部验收边界 | 保持 | 生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 Linux x86_64/aarch64 发布产物矩阵收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 安装脚本和部署文档支持 x86_64/aarch64，但 release workflow 只生成 x86_64 artifact，ARM64 生产部署缺少可复现 CI 来源 |
| 工作流修复 | 已完成 | `.github/workflows/release.yml` 改为 x86_64 + aarch64 matrix，分别使用 `ubuntu-latest` 与 `ubuntu-24.04-arm`，artifact 名称和 target triple 独立隔离 |
| 单元护栏 | 已完成 | `scripts/test_release_workflow_unit.py` 锁定两个 runner、两个 target 和动态 artifact 校验；未把任何凭据写入 workflow |
| 外部验证边界 | 保持 | 本地只能验证 YAML/脚本合同；真实 GitHub Actions runner、Linux artifact 下载和目标主机安装仍需触发 workflow 后验收 |

## 2026-07-27 Linux artifact 实际目标架构校验收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | matrix target 只参与 artifact 命名，原构建命令未显式 `--target`，打包器固定读取 `target/release`，存在 runner 与文件名漂移时的误标风险 |
| 工作流修复 | 已完成 | 按 matrix target 编译，打包器读取对应 `target/<triple>/release` binary，并同时校验 `uname -m` 与 `rustc` host triple |
| 单元护栏 | 已完成 | release workflow 和 package-release unit 锁定 machine、target、binary path override 与实际 target build 命令 |
| 外部验证边界 | 保持 | 本地已验证文本合同；真实 GitHub Actions runner、artifact 下载/执行和生产主机安装仍待 workflow 发布后验收 |

## 2026-07-27 生产 readiness 配置值域收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `production-readiness.sh` 原先只拒绝开发开关值为 `1`，未知值以及非 HTTPS CORS origin 可能通过配置门禁 |
| 脚本修复 | 已完成 | CORS 强制单一 HTTPS origin；`LOBSTER_DEV_AUTH_BYPASS` 与 `LOBSTER_DEV_EMAIL_OTP_INLINE` 只允许 `0` 或未设置 |
| 防回归 | 已完成 | production-readiness unit 增加有效配置、HTTP CORS 和未知开发开关的实际脚本执行断言；bash syntax 与 release gate unit 通过 |
| 外部验证边界 | 保持 | 仍需在目标 Linux 主机用真实 env、Nginx、域名/TLS 和邮件 Webhook 执行线上验收 |

## 2026-07-27 H5 owner-only 场景入口认证边界收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | 真实 Gateway 页面把默认/持久化身份当成已登录，且住宅 rail 的 `display:flex !important` 会覆盖普通 inline 隐藏，未登录时场景编辑器入口仍可能可见 |
| H5 修复 | 已完成 | Gateway 页面要求当前居民身份同时持有 session token；无 Gateway 的静态预览继续支持 `identity` 视觉验收；owner-only 隐藏使用可逆的 `display:none !important`，认证 refresh/401/403 后重新投影 |
| 真实浏览器验证 | 已通过 | 真实 Gateway 页面无 token 实际 `display:none`，带身份与 session token 实际 `display:flex`；登录流程和住宅页 CSS 覆盖均复核 |
| 回归验证 | 已通过 | Web 1391/1391；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0，Gateway 310、TUI 233、双浏览器、认证、居民主链、Provider federation 与发布门禁通过 |
| 外部验收边界 | 保持 | 生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 TUI Gateway shell state/scene render 合同接入

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | TUI 启动原来只调用本地 `bootstrap_conversations` 与 `launch_seed`；即使配置 `LOBSTER_WAKU_GATEWAY_URL`，也没有读取 `/v1/shell/state`，未消费 Gateway 的 `conversation_shell`/`scene_render` 居民可见合同 |
| TUI 接入 | 已完成 | Gateway 配置下以 `conversation_shell` 为会话来源，合并 `scene_render` 场景字段，导入消息及 edit/recall 元数据；无 Gateway 时才保留离线 seed |
| 命令兼容 | 已完成 | `/open` 先保持模式内稳定种子会话槽位，再追加 Gateway 动态会话，避免新增服务端会话改变既有快捷序号 |
| 回归验证 | 已通过 | TUI 233/233；真实 resident smoke 使用 Bearer 验证 Gateway shell state hydration 并断言 `TUI_GATEWAY_BOOTSTRAP_SMOKE`；完整 release gate 通过，Gateway 310/Web 1391 |
| 外部验收边界 | 保持 | 生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 TUI edit/recall Gateway shell 合同收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | TUI `/edit` 与 `/recall` 请求仍发送旧 `sender` 字段，缺少 Gateway 正式要求的 `room_id` 与 `actor`；旧单测固定了错误请求体，因此未暴露运行时问题 |
| TUI 修复 | 已完成 | 请求构造统一发送 `room_id`、`message_id`、`actor`，调用处使用当前活动会话 ID；不改变本地消息发布路径 |
| 回归验证 | 已通过 | TUI 233/233；`SKIP_BUILD=1 scripts/smoke-shell-direct-http.sh` 真实 Gateway edit/recall/SSE 链路通过；完整 release gate 通过，Gateway 310/Web 1391 |
| 外部验收边界 | 保持 | 生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 仍待目标环境声明与线上验收；未提交 Git |

## 2026-07-27 本地发布产物与安装前门禁准备

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 发布门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 310/310、Web 1391/1391、双浏览器、Provider federation、终端 smoke 与生产 panic scan 均通过 |
| 本地打包 | 已通过 | `scripts/package-release.sh` 成功生成当前 worktree 的 source、H5 与 `aarch64-apple-darwin` Gateway 产物；tar listing 完整且未发现凭据文件名 |
| Linux 安装前验证 | 已通过 | `bash scripts/smoke-install-layout.sh` 通过；安装器、release workflow、production-readiness、package 与脚本语法单元测试通过 |
| 生产边界 | 未完成 | macOS 产物不能安装到 Linux；Linux artifact 需通过 `.github/workflows/release.yml` 生成。Atlas 当前没有 lobster-chat 生产环境声明，正式主机、域名/TLS、真实邮件 OTP 和公网双端验收仍待确认/执行 |

## 2026-07-27 admin-ds 居民制裁解除真实操作闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 后台操作 | 已完成 | 居民制裁列表对 `Active` 制裁显示“解除制裁”按钮，调用 `/v1/admin/residents/unsanction`，成功后刷新安全治理投影；无 Gateway、HTTP 失败和网络异常均有明确反馈 |
| 前端护栏 | 已完成 | 新增 admin-ds 静态端点/函数断言与运行时 POST、身份字段、无 Gateway 提前返回测试 |
| 当前验证 | 已通过 | Web `npm test`：1391/1391；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 退出码 0；Gateway 310/310、双浏览器、Provider federation、终端 smoke 与发布门禁均通过 |
| 外部验收边界 | 保持 | 本轮只完成本地代码与发布链验证；生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 和 admin 线上演练仍未执行；未提交 Git |

## 2026-07-27 Gateway 管理房间冻结跨重启持久化

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `/v1/admin/rooms/freeze` 与 `/unfreeze` 只修改 `public_rooms` 内存状态，未写回 `governance-state.json`；Gateway 重启后 `is_frozen` 会丢失 |
| Gateway 修复 | 已完成 | 冻结/解冻成功后持久化 governance state；写盘失败时回滚内存变更并返回错误 |
| 回归测试 | 已完成 | `admin_room_freeze_persists_across_restart` 覆盖 freeze、重启读取、unfreeze、再次重启读取 |
| 当前验证 | 已通过 | Gateway 305/305；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过 |

## 2026-07-27 Gateway 管理后台操作持久化错误合同收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 覆盖范围 | 已完成 | 邀请码、日志处理、权限组/授权、设备管理、房间成员和消息审核均纳入统一写盘错误处理 |
| Gateway 修复 | 已完成 | 持久化失败返回错误并回滚内存；房间成员写入不再吞掉 timeline store 错误；设备错误带持久化失败上下文 |
| HTTP 合同 | 已完成 | admin 写操作不再把持久化失败当作成功响应；保留参数/资源错误的原有 4xx 语义 |
| 回归测试 | 已完成 | 新增 `admin_ops_persist_across_restart`，覆盖 invite/log/moderation/permission/room-member/device 跨重启读取 |
| 当前验证 | 已通过 | Gateway 309/309；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过，Web 1389/1389、双浏览器、认证、CLI/TUI 与 provider federation smoke 均通过 |

## 2026-07-27 Gateway 治理、昵称与设备绑定跨重启持久化

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 治理修复 | 已完成 | 世界解制裁现在写回 `governance-state.json`，写盘失败恢复原制裁状态 |
| 身份修复 | 已完成 | admin/shell 昵称写盘失败回滚内存，避免返回错误后状态仍被半修改 |
| 设备修复 | 已完成 | OTP 绑定设备同步写回 `device-state.json`，失败时同时恢复 binding 与设备投影 |
| 回归测试 | 已完成 | 新增 `governance_profile_and_device_binding_persist_across_restart`，覆盖昵称、制裁、解制裁和设备绑定跨重启读取 |
| 当前验证 | 已通过 | Gateway 310/310；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过，Web 1389/1389、双浏览器、认证、CLI/TUI 与 provider federation smoke 均通过 |

## 2026-07-27 Gateway 管理系统配置跨重启持久化

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `/v1/admin/config` 只更新运行时 `app_config`，Gateway 重启后配置恢复为空 |
| Gateway 修复 | 已完成 | 新增 `app-config.json` 的 atomic write/load；配置写盘失败时回滚内存并返回错误 |
| HTTP 合同 | 已完成 | 配置持久化失败返回 HTTP 500，成功路径仍记录 `admin:config` 审计事件 |
| 回归测试 | 已完成 | `admin_config_persists_across_restart` 与 6 条 admin config 聚焦测试通过 |
| 当前验证 | 已通过 | Gateway 308/308；config focused 6/6；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过，Web 1389/1389、双浏览器、认证、CLI/TUI 与 provider federation smoke 均通过 |

## 2026-07-27 Gateway 管理居民/制裁操作跨重启持久化

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 根因 | 已确认 | `admin_create_resident()`、`admin_ban_resident()` 和 `admin_unban_resident()` 修改注册/制裁内存后未持久化；重启会丢失后台操作结果 |
| Gateway 修复 | 已完成 | 新建居民写回 `auth-state.json`；封禁/解封写回 `governance-state.json`；写盘失败回滚内存并返回错误 |
| HTTP 合同 | 已完成 | 新建居民持久化失败返回 HTTP 500，不再把失败误报为重复居民 409 |
| 回归测试 | 已完成 | 新增居民和制裁/解封跨重启测试；admin 聚焦 57/57 通过 |
| 当前验证 | 已通过 | Gateway 309/309；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过 |

## 2026-07-27 app.js 技术债继续收口: 会话总览 context/status 运行态接线

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| overview context adapter | 已完成 | 新增 `conversationOverviewContextModelForRoom()`；`createConversationOverviewContextNode()` 只负责 DOM 投影，summary/context/status 运行态由 adapter 收集后交给 `conversationOverviewContextModel()` |
| overview base status adapter | 已完成 | 新增 `conversationOverviewBaseStatusPillsForRoom()`；`gatewaySyncController`、`roomSendErrors`、pending echo、未读和会话摘要均在 adapter 边界注入，DOM appender 不再自行决定 pill 文案与 tone |
| user status adapter | 已完成 | 新增 `userConversationStatusPillsForRoom()`；user status DOM 与运行态状态模型分离，保留 quick action DOM 控件在 app.js |
| TDD 护栏 | 已完成 | 更新 `shell-pages-static.test.mjs`，分别锁定 DOM renderer 与运行态 adapter 边界；focused suite 246 passed / 0 failed |
| Web 全量验证 | 已通过 | `npm test`：1385 passed / 0 failed；layout、frontend realness、`node --check` 与 `git diff --check` 通过 |
| 完整发布门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 通过；Gateway 304、Rust workspace、CLI/TUI、认证、居民主链、双 HTTP、Web shell、双浏览器、provider federation、安装布局、发布工作流、生产 readiness 和完整验证均通过 |
| 外部验收边界 | 保持 | 本轮只验证本地代码与发布链；生产 Linux 主机、正式域名/TLS、真实邮件 OTP、公网双端 IM 和账号申诉演练仍未执行；未提交 Git |

### 下一步建议

1. 进入目标 Linux 主机：使用发布 artifact 部署 Gateway/H5，配置 systemd、Nginx、正式 CORS/TLS 与生产邮件 Webhook。
2. 运行 `scripts/production-readiness.sh` 与 `scripts/smoke-public-ingress.sh`，再按 `docs/DEPLOYMENT.md` 完成真实邮箱 OTP、双端公共/私聊发送编辑撤回、失败重试、重启恢复和 admin 验收。

## 2026-07-27 app.js 技术债继续收口: Gateway shell state 标准化下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| shell state normalization | 已完成 | 新增 `normalizeShellStateForState(payload, fallbackState)` 到 `shell-state-normalize.js`，统一处理 Gateway conversation contract、legacy rooms 与 fallback clone |
| app.js 接线 | 已完成 | `loadShellState()`、Gateway shell state 应用和 IndexedDB cache 恢复均调用共享纯函数；app.js 删除本地 contract merge 实现，约减少 13 行 |
| TDD 护栏 | 已完成 | 新增 fallback clone、contract 覆盖 legacy、legacy-only 三类单测，并增加 app.js 静态接线/禁止本地 normalization 回流测试 |
| 验证 | 已通过 | state-normalize + pages static focused suite 183 passed / 0 failed；当前 Web 全量 `npm test` 1389 passed / 0 failed，layout、frontend realness、相关 JS syntax 与 `git diff --check` 通过 |

## 2026-07-26 app.js 技术债继续收口: 会话总览头部纯规格下沉

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 任务选择 | 已确认 | 按 2026-07-16「继续按模块边界收口剩余前端技术债」与旧队列「conversation overview 非用户状态/动作 specs」建议，先收会话总览头部 |
| overview header 纯规格 | 已完成 | 新增 `conversationOverviewHeaderModel()`（`shell-room-render.js`），下沉 user/admin/world 摘要回退链、direct accent 徽章、紧凑壳身份徽章隐藏、场景横幅与管家徽章顺序 |
| app.js 委托接线 | 已完成 | `createConversationOverviewHeaderNode()` 只保留 DOM 创建；新增 `conversationOverviewHeaderModelForRoom()` 收集 `roomThreadHeadline/roomSummaryLine/roomAudienceLabel/caretakerProfile` 等运行态后委托纯模型，未外提这些实时状态函数本体（保持旧队列的参数化边界告诫） |
| TDD 护栏 | 已完成 | 先红后绿：`shell-room-render.test.mjs` 新增 7 条模型测试；`shell-pages-static.test.mjs` 新增委托护栏（禁止 `后台对象 ·`、summary 回退链和 direct tone 三元回流 app.js），并同步更新旧 header DOM 护栏为委托形状 |
| app.js 规模 | 已更新 | 8423 → 8417 行 |
| 验证 | 已通过 | `node --test test/shell-room-render.test.mjs test/shell-pages-static.test.mjs` 227 passed / 0 failed；`npm test` unit 1366 passed / 0 failed（原 1358 + 新增 8），layout 与 frontend realness 通过；`node --check app.js shell-room-render.js`、`git diff --check` 通过 |
| 未验证边界 | 保持 | 生产主机、域名、真实邮件 OTP 复验仍需外部环境，本轮未触碰；未提交 Git |

### 下一步建议

1. 继续 conversation overview 收口：`createConversationOverviewContextNode()` / `appendConversationOverviewBaseStatusPills()` 系列可按同样模型化模式下沉（注意 `gatewaySyncController`、`roomSendErrors` 等运行态需以参数注入）。
2. `createUserConversationStatusNode()` 的用户态状态徽章与之共享 pill 语义，可在 context/status 收口后合并为同一模型家族。

## 2026-07-16 app.js 技术债恢复推进: 生命周期、输入附件、消息搜索与回显状态收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 拆分恢复授权 | 已确认 | 用户恢复 `app.js` 技术债推进；继续保持 Gateway 合同为真值来源，不混入视觉改版和产品扩展 |
| Quick Action reader 状态边界 | 已完成 | `shell-quick-action-reader.js` 从模块级可变回调 + `initQuickActionReaders()` 改为 `createQuickActionReaders(deps)` 实例工厂，消除初始化顺序和跨实例状态污染 |
| 新模块直接测试 | 已完成 | 新增 Quick Action reader 实例隔离测试和 conversation callout renderer DOM 投影测试；本轮六个 WIP 模块均已有直接测试 |
| 房间画像侧栏 renderer | 已完成 | 画像 canvas 保留、旧节点清理、摘要与状态 chip DOM 投影下沉到 `shell-scene-chrome.js`，`app.js` 只组装模型和依赖 |
| 消息搜索 chrome | 已完成 | 搜索栏/按钮创建、插入和 toggle 接线下沉到现有 `shell-message-search.js`；安全 DOM 静态契约改为验证模块接线，不再依赖旧内联注释 |
| Shell 生命周期边界 | 已完成 | 新增 `shell-lifecycle.js`，统一启动顺序与 visibility/focus/pageshow 前台刷新监听；`app.js` 只注入启动阶段和刷新回调 |
| 输入附件控制器 | 已完成 | 新增 `shell-composer-symbols.js`，颜文字分类 tabs、插入/提及、菜单开关和文档点击关闭统一由实例控制器管理，并提供模块级直接测试 |
| 消息搜索运行时控制器 | 已完成 | `shell-message-search.js` 统一动态 Gateway/房间读取、300ms 防抖、过期响应抑制、安全结果渲染、消息定位高亮与关闭清理；`app.js` 仅保留实例接线 |
| Pending echo 实例状态 | 已完成 | `shell-message-state.js` 新增 `createPendingMessageEchoStore()`，统一发送回显的入队、失败标记、删除、按房间/全量清理和快照读取；实例隔离测试锁定不再共享或直接改写 `app.js` 全局对象 |
| Gateway 实时同步控制器 | 已完成 | 新增 `shell-gateway-realtime.js` 实例控制器，统一持有 `EventSource`、重连 timer、快照版本、错误降级与轮询切换；`app.js` 只注入 Gateway 状态读写和渲染回调，不再持有 SSE 生命周期全局变量 |
| 消息发送状态控制器 | 已完成 | 新增 `shell-message-send.js`，实例化持有发送中状态并统一本地发送、Gateway POST、强制刷新、pending echo 清理和失败分类；源头抑制并发重复调用，POST 已成功但刷新失败时继续保留“可能已发出”语义 |
| Gateway 轮询与前台恢复控制器 | 已完成 | 新增 `shell-gateway-polling.js`，实例化持有 fallback interval、前台刷新并发闸门和 1200ms 节流；实时控制器与生命周期控制器只通过注入调用，`app.js` 不再持有轮询 timer 和前台刷新时间戳全局状态 |
| Gateway 同步状态与刷新控制器 | 已完成 | 新增 `shell-gateway-sync.js`，统一持有刷新中、最近成功时间和错误状态，并编排 world/shell/provider 三路刷新及最终 UI 回调；轮询、SSE、手动刷新和本地发送回显复用同一实例，`app.js` 不再共享三个同步状态全局变量 |
| shell-state 生产接线 | 已完成 | 修复此前只抽模块和单测、生产 `app.js` 仍保留重复实现的问题；workspace、聊天窗格恢复、草稿、已读标记、快捷状态与快照现已委托 `shell-state.js`。`resolveChatPaneMode` 支持调用方响应式 fallback，保持窄屏 `list/thread` 行为 |
| 认证状态实例控制器 | 已完成 | `shell-auth.js` 新增 `createAuthController()`；H5 主 shell 与 standalone 登录页分别持有 challenge、session token、DOM refs 和回调，不再共享模块级认证状态。新增双实例隔离测试，并保留旧导出作为兼容单例入口 |
| 聊天专注实例控制器 | 已完成 | 新增 `shell-chat-focus.js`，实例化持有偏好、当前激活态与 toggle DOM；workspace 切换仅暂停专注而不覆盖用户偏好，`app.js` 只注入存储、当前 workspace、锚点和 badge 回调。4 个直接测试及静态生产接线契约锁定实例隔离与真实接入 |
| 生产邮件 OTP 适配器 | 已完成 | 新增 HTTPS Webhook mailer；生产模式未配置或投递失败时回滚 challenge 并 fail-closed，Bearer 凭证仅走 header。release 二进制黑盒验证缺配置返回 400，配置本地捕获 Webhook 后 request → delivery → verify 成功 |
| OTP 外部投递锁隔离 | 已完成 | Gateway 只在运行时锁内完成 OTP 校验、challenge 创建和持久化，HTTPS 邮件投递在锁外执行；慢 mailer 不再阻塞其他 Gateway 请求，投递失败会重新加锁并精确撤销 challenge 与本次限流记录 |
| 注册账号后台审计 | 已完成 | `/v1/admin/residents` 以注册记录为基础投影，未入城账号也可见；返回脱敏邮箱、注册状态、注册/验证/最近登录时间，admin-ds 已切换到该管理端点并在详情中展示 |
| 生产接管与申诉手册 | 已完成 | 重写 `docs/DEPLOYMENT.md` 为当前 304/1358 基线，补齐 artifact、systemd 环境文件、TLS、公网/真实邮箱/双端 IM 验收、备份恢复和回滚；新增 `ACCOUNT_APPEAL_RUNBOOK.md`，明确只读调查、普通制裁恢复、世界黑名单升级和审计留痕边界 |
| app.js 当前规模 | 已更新 | 8942 → 8422 行；已低于旧 `<8700` 参考值，但后续仍只按职责边界和直接测试推进 |
| 多来源改动外盘快照 | 已完成 | 排除可重建的 `target`、node_modules 和临时输出后，将源码/文档/测试/必要资产完整镜像到 `/Volumes/AJW-Data/Backups/lobster-chat-pre-final-20260716/source`，另保存 binary diff；checksum dry-run 无差异 |
| Web 全量验证 | 已通过 | `npm test`：1358 tests / 0 failed，layout 与 frontend realness 通过；首次全量出现一次 Hub 消息行时序抖动，目标用例单独连续 10 次及第二轮全量均通过，未放宽断言 |
| 双浏览器真实链路 | 已通过 | `SKIP_BUILD=1 npm run smoke:dual-browser` 验证双用户发送、编辑、撤回、503 故障注入与失败重试闭环 |
| 完整发布门禁 | 已通过 | 场景编辑器统一画布与比例规范、room digest 统计投影下沉后执行 `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh`；Gateway 304、Web 1358、Rust workspace、CLI/TUI、鉴权、居民主链、HTTP、双浏览器、provider 联邦和交付脚本全部通过 |
| 单城 IM 完成度审计 | 已完成 | P0/P2/P4 代码与自动化门禁已闭环；旧 DS 清单中的 admin 写端点、session/logout、OTP 限流和审计均已实现。当前真实外部缺口是生产主机/域名/邮件适配器环境复验，不在本地代码中伪造完成 |
| 下一阶段 | 待推进 | 继续按模块边界收口剩余前端技术债；生产验收和账号申诉手册已就绪，真实上线复验仍需目标主机、域名和邮件发送配置 |
| Linux x86_64 发布工作流 | 已完成 | 新增手动触发的 `.github/workflows/release.yml`；先跑 Rust/Web 全量测试，再上传目标架构 Gateway、H5、源码和 SHA256SUMS，避免在 macOS 上误用 Darwin binary |
| Nginx Bearer 透传 | 已完成 | 安装脚本生成配置显式透传 `Authorization`，新增安装合同测试和 install-layout smoke，避免生产代理依赖默认 header 行为 |
| 生产就绪只读检查 | 已完成 | 新增 production-readiness.sh，检查正式 CORS、关闭开发鉴权/inline OTP、HTTPS 邮件 Webhook、Bearer 配置；可选探测公网 health、provider 与 CORS，不读取或输出密钥；已纳入 release gate quick unit |

## 2026-07-16 P1 场景编辑器坐标画布收口

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 统一逻辑画布 | 已完成 | `scene-editor.html` 增加自适应 16:9 `scene-canvas`；背景、热点和缩放共同挂在画布内，拖拽/缩放坐标改用画布 rect，移动端不再以 stage 留白计算热点 |
| 比例字段规范 | 已完成 | `aspect_ratio_permyriad` 明确为高度/宽度 × 10000，16:9 规范值为 5625；Gateway 默认场景、chat-core、storage fixture、admin-ds、编辑器和主合同 fixture 已统一 |
| P1 真实验收 | 已通过 | 新增静态画布契约与移动 viewport bounding-box 检查；Web `npm test` 通过 1358/1358，包含 layout 与 frontend realness |
| 设计与实施记录 | 已完成 | [设计记录](superpowers/specs/2026-07-16-scene-editor-coordinate-canvas-design.md) 与 [实施计划](superpowers/plans/2026-07-16-scene-editor-coordinate-canvas.md) 已写入；未提交 Git commit，保留现有工作区变更 |

## 2026-07-14 发布门禁补强: 非 SKIP_BUILD 路径执行 workspace Rust tests

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| release gate Rust 测试 | 已完成 | 非 `SKIP_BUILD=1` 路径在 fmt 后、clippy/build 前新增 `cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --quiet` |
| 门禁契约测试 | 已完成 | `test_smoke_release_gate_unit.py` 锁定执行顺序，避免只 lint/build 不跑 workspace tests |
| 完整构建门禁 | 已通过 | `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 全流程通过 |

## 2026-07-14 Gateway 写鉴权矩阵补齐: 个人房间与居民关系

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 个人房间写路由 | 已完成 | 新增缺失 Bearer 回归，覆盖 `/v1/personal-room` 与 `/v1/personal-room/access-policy` |
| 居民关系写路由 | 已完成 | 新增缺失 Bearer 回归，覆盖关系 request/accept 两条写路由 |
| Shell 昵称写路由 | 已完成 | 将 `/v1/shell/nickname` 纳入 direct/shell 写路由鉴权矩阵 |
| Gateway 验证 | 已完成 | Gateway 298 passed / 0 failed；fmt、Gateway clippy、`git diff --check` 通过 |
| 前端冻结边界 | 保持 | 未修改 `apps/lobster-web-shell/app.js` |

## 2026-07-14 发布门禁收口: Web 双浏览器 smoke 诊断与 fake-dom 模块契约

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| Web 双浏览器 smoke | 已完成 | 真实 console/pageerror/requestfailed 现在会使 smoke 失败；消息重试场景故意返回的 503 被明确标记为 expected，不再造成误报 |
| fake-dom 模块契约 | 已完成 | `APP_LOCAL_IMPORT_PATHS` 补齐 `shell-quick-action-reader.js`，修复 app.js 新模块导入在 Node fake-dom 测试中的临时模块解析失败 |
| app.js 边界 | 保持冻结 | 本轮未修改 `apps/lobster-web-shell/app.js` |

## 2026-07-14 继续推进: 城市/世界治理写鉴权回归矩阵

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 城市/世界治理写 Bearer 回归矩阵 | 已完成 | 新增 `city_and_governance_write_routes_require_bearer_session_without_dev_bypass`，覆盖城市创建/加入/审批/执事/联邦策略/公共房间及世界公告、安全治理、制裁/解制裁写入；生产模式缺失 Bearer 均断言 401 |
| Gateway 聚焦验证 | 已完成 | Gateway：297 passed / 0 failed；fmt、clippy、`git diff --check` 通过 |
| 前端冻结边界 | 保持 | 未修改 `apps/lobster-web-shell/app.js`，未改变 H5 合同 |

## 2026-07-14 继续推进: Gateway 核心管理写鉴权回归矩阵

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 核心 admin 写路由 Bearer 回归矩阵 | 已完成 | 新增 `core_admin_write_routes_require_bearer_auth_without_dev_bypass`，覆盖居民 ban/unban/nickname、房间 freeze/unfreeze、配置、消息审核、邀请码、房间成员和场景写入；生产模式缺失 Bearer 均断言 401 |
| Gateway 聚焦验证 | 已完成 | `cargo test -p lobster-waku-gateway --quiet`：296 passed / 0 failed；`cargo fmt --all -- --check`、Gateway clippy、`git diff --check` 通过 |
| 前端冻结边界 | 保持 | 未修改 `apps/lobster-web-shell/app.js`，未改变 H5 合同 |

## 2026-07-13 继续推进: Gateway 审计持久化、管理鉴权与 CI 门禁

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js DS 拆分 | 暂停 | 按用户要求不再继续拆 `app.js`；保留现有模块与测试，不在本轮扩大边界 |
| Rust CI 质量门禁 | 已完成 | `.github/workflows/ci.yml` 增加 `cargo fmt --all -- --check` 与 `cargo clippy --workspace -- -D warnings` |
| Gateway 审计事件 ID | 已完成 | `audit-{timestamp_ms}-{sequence}`，同一毫秒及重启后仍保持唯一序列 |
| 审计日志写入 | 已完成 | `audit-log.json` 改用共享 `atomic_write_file`，避免半写文件 |
| 管理接口读鉴权 | 已完成 | `/v1/admin/*` 敏感 GET 在关闭 dev bypass 时校验真实 Bearer 会话；能力目录保留公开 |
| 管理写接口身份/能力鉴权 | 已完成 | 管理写操作校验会话 actor 与 body `actor_id` 一致，并按操作检查能力；省略 actor 的前端请求从会话派生 |
| 设备与高风险管理写鉴权 | 已完成 | 设备增删/封禁、居民/权限组/制裁写操作均有 Bearer 门禁；设备操作要求 `admin:diagnostics` |
| Provider/mirror 写接口 Bearer 门禁 | 已完成 | `/v1/provider/connect`、`/v1/provider/disconnect`、`/v1/world-mirror-sources` 统一走管理 Bearer 校验；生产模式拒绝缺失/非法会话，dev fixture bypass 保留 |
| Export Bearer 身份绑定 | 已完成 | `/v1/export` 将 `resident_id` 绑定到 Bearer 会话；CLI export 必须使用 session token，避免仅凭 query 身份导出私聊历史 |
| CLI scoped GET Bearer parity | 已完成 | `/v1/cli/inbox`、`/v1/cli/rooms`、`/v1/cli/tail` 按 user session 或 agent sidecar token 校验身份；CLI 读取命令支持 `--token` / `--agent-token`，不再把无 token 请求送到 Gateway |
| CLI read/presence Bearer parity | 已完成 | CLI `read` 与 `presence` 现在必须透传当前 user session；无 token 在本地先提示 login，agent sidecar 对 resident state 明确拒绝，避免无认证 POST |
| CLI admin read Bearer parity | 已完成 | `config --get`、`residents`、`rooms-admin` 透传 user session Bearer；无 token 在本地提示 login，agent sidecar 不得冒充管理读取 |
| CLI scoped search Bearer parity | 已完成 | CLI search 新走 `/v1/cli/search`，按 user/agent identity 过滤可见会话；无 token 本地提示 login；旧 H5 搜索当时暂保持兼容路径，现已由 2026-07-27 最新区块补齐认证与可见性 |
| TUI scoped search parity | 已完成 | TUI 新增 `/search <关键词>`，按当前会话通过 `/v1/cli/search` 查询并沿用 session/agent Bearer；`app.js` 保持冻结 |
| TUI scoped search resident smoke | 已完成 | resident mainline smoke 真实执行 TUI `/search`，捕获脚本后的终端投影并断言当前私聊命中；`app.js` 保持冻结 |
| web-shell smoke 完整测试集 | 已完成 | 移除 `--test-force-exit`，避免 smoke 在退出前静默少跑测试；当前 smoke 与 npm unit 均执行 1310 tests |
| Gateway 生产启动空时间线 | 已完成 | 默认生产配置不再自动写入 demo 聊天记录；仅测试构建或显式 `LOBSTER_DEV_AUTH_BYPASS=1` fixture seed，`app.js` 保持冻结 |
| resident-scoped shell state/SSE Bearer 绑定 | 已完成 | `/v1/shell/state` 与 `/v1/shell/events` 带 `resident_id` 时必须匹配 Bearer session；缺失、错配均 401，未触碰 `app.js` |
| 城市/世界治理写接口身份一致性 | 已完成 | 城市写入统一校验 body actor 与 Bearer 会话身份；世界广场/安全治理写入拒绝缺失会话，保留 dev bypass 仅供本地 fixture |
| Direct/Shell 写接口身份门禁 | 已完成 | direct/open、消息发送/编辑/撤回、场景、presence/read 均校验 Bearer actor；合成 qa-* smoke fixture 显式开启 dev bypass，生产模式缺失会话回归已覆盖 |
| CLI sidecar/user 写操作认证 | 已完成 | `/v1/cli/send`、shell edit/recall 对 `agent:<id>` 使用 `LOBSTER_AGENT_TOKENS` 按身份绑定 token；`user:<id>` 复用 resident session；`lobster-cli send/edit/recall` 支持 `--token` / `LOBSTER_SESSION_TOKEN` / `--agent-token` / `LOBSTER_AGENT_TOKEN`，CLI 请求保留并校验 typed `actor_address`，合成 smoke 显式开启 dev bypass |
| admin-ds 会话透传 | 已完成 | admin-ds GET/POST 自动透传 `lobster-session-token`，与 Gateway 管理鉴权闭环 |
| admin-ds 次级真实数据空态 | 已完成 | Gateway 已连接时邀请码和审计日志的空/失败响应清空旧 mock，不再把本地演示数据伪装成正式数据；无 Gateway 的本地预览仍保留 mock |
| admin-ds 主投影真实空态 | 已完成 | Gateway 已连接时居民、房间、消息的有效空数组覆盖旧 mock；HTTP/畸形响应清空主投影并显示“部分同步”，避免 Gateway 页面伪装成本地演示 |
| admin-ds 权限组真实空态 | 已完成 | Gateway 已连接时权限组列表严格使用 `/v1/admin/permission-groups`；空/失败不再回退四类内置展示 mock，离线本地预览仍保留内置说明 |
| admin-ds 房间加入规则真实来源 | 已完成 | Gateway 当前未提供房间加入规则合同；移除四条硬编码开放/邀请/白名单状态，改为明确的待接入空态，避免后台伪装正式策略 |
| admin-ds 世界公告与安全治理真实空态 | 已完成 | 世界公告、安全通告、举报、居民制裁读取失败或畸形响应时清空旧投影并渲染空态，不再继续展示过期治理数据 |
| admin-ds 系统服务健康真实来源 | 已完成 | Gateway 当前未提供服务健康明细合同；移除公网 Gateway/SMTP/AI/配额/存储/Webhook 硬编码状态，改为明确待接入空态 |
| admin-ds 仪表盘事件真实来源 | 已完成 | 未连接 Gateway 时移除 resident_demo/IP/AI 等硬编码审计样例，显示无实时事件空态；Gateway 成功时继续由审计投影填充 |
| smoke 运行时输入契约 | 已完成 | 统一校验 `SKIP_BUILD`、`GATEWAY_BIN`/`CLI_BIN`/`TUI_BIN` 与 provider artifact 的边界；release gate 不再把 provider-only `BIN_PATH` 泄漏到普通 smoke |
| TUI Gateway Bearer parity | 已完成 | TUI 的 direct/open、编辑、撤回、管理读写和治理查询统一透传 `LOBSTER_SESSION_TOKEN` / `LOBSTER_AGENT_TOKEN`；resident smoke 显式传入 session token |
| provider federation smoke 鉴权契约 | 已完成 | 合成上下游 Gateway 显式开启 `LOBSTER_DEV_AUTH_BYPASS=1`，避免无 token 的 `smoke-bot` 被生产 Bearer 门禁误判；真实生产鉴权逻辑不变 |
| provider federation smoke 端口隔离 | 已完成 | 未显式传入端口时自动预留两个不同的本机临时端口；保留 `UPSTREAM_PORT`/`DOWNSTREAM_PORT` 覆盖能力，避免并行 smoke 固定端口冲突 |
| resident mainline smoke 端口隔离 | 已完成 | 未显式传入 `PORT` 时自动预留本机临时端口；保留显式端口覆盖，两个并行 resident smoke 已实测同时通过 |
| CLI channel smoke 端口隔离 | 已完成 | 未显式传入 `PORT` 时自动预留本机临时端口；保留显式端口覆盖，两个并行 CLI send/edit/recall smoke 已实测同时通过 |
| auth registration smoke 端口隔离 | 已完成 | 未显式传入 `PORT` 时自动预留本机临时端口；保留显式端口覆盖，两个并行 OTP/黑名单 smoke 已实测同时通过 |
| shell dual HTTP smoke 端口隔离 | 已完成 | 未显式传入 `PORT` 时自动预留本机临时端口；保留显式端口覆盖，两个并行 shell state/events smoke 已实测同时通过 |
| shell direct HTTP smoke 端口隔离 | 已完成 | 未显式传入 `PORT` 时自动预留本机临时端口；保留显式端口覆盖，两个并行 direct open/send/edit/recall smoke 已实测同时通过 |
| start-terminal 外部 Gateway 契约 | 已完成 | 显式设置 `LOBSTER_WAKU_GATEWAY_URL` 时仅复用该外部 Gateway；目标不可达立即 fail-fast，不再错误地按本地 `HOST:PORT` 启动后等待错误地址 |
| terminal smoke 本地代理隔离 | 已完成 | `/health` 探针强制补齐 `NO_PROXY/no_proxy=127.0.0.1,localhost`；即使宿主带 `ALL_PROXY`，终端 smoke 仍直接访问本地 Gateway |
| TUI `/dm` Gateway fail-closed | 已完成 | 已配置 Gateway 但 `/v1/direct/open` 失败时不再静默创建本地直聊；仅未配置 Gateway 时保留离线兼容回退 |
| TUI `/dm` 空响应 fail-closed | 已完成 | Gateway 返回空白 `conversation_id` 时拒绝创建无效私聊会话，避免响应合同畸形被当作成功 |
| provider federation smoke 本地代理隔离 | 已完成 | 直接运行 provider smoke 时也强制 `NO_PROXY/no_proxy=127.0.0.1,localhost`，避免本地上下游 Gateway 健康/状态请求被宿主代理接管 |
| 其余本地 smoke/预览代理隔离 | 已完成 | auth、CLI、resident、shell dual/direct、start-terminal、restart-gateway、start-web-preview 统一追加本地代理 bypass；公网 ingress 不改 |
| 发布门禁复验 | 已通过 | Gateway 294 tests、CLI 119 unit + 24 integration tests、TUI 230 tests、workspace 全绿、web-shell 1310 tests、fmt/clippy、CLI/auth/resident/shell/web smoke 全部通过；provider federation 已用显式 `BIN_PATH` 一并验证 |

### 本轮下一步

继续做不依赖 `app.js` 的 Gateway/TUI/CLI 与发布鲁棒性收口；任何前端 DS 拆分先暂停，除非用户重新授权。

### 2026-07-13 TUI `/dm` 空响应 fail-closed 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 注入式 `/v1/direct/open` 成功回调若携带空白 `conversation_id`，TUI 原先会继续构造并持久化无效私聊会话 |
| 修复 | 已完成 | `resolve_direct_conversation_id()` 在 Gateway 路径校验 ID 非空；无 Gateway 的离线 canonical fallback 保持不变 |
| 防回归 | 已完成 | 新增空白 `conversation_id` 测试，锁定明确错误文案；既有 Gateway 失败与离线 `/dm` 测试继续保留 |
| 验证 | 已通过 | TDD 红灯后绿灯：`cargo test -p lobster-tui dm_gateway_ --quiet`、`cargo test -p lobster-tui`（228 passed）、workspace、fmt/clippy 与含 provider federation 的 release gate 均通过 |

### 2026-07-13 provider federation smoke 本地代理隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | provider federation smoke 直接执行时未设置本地代理 bypass；宿主配置 `ALL_PROXY/HTTP_PROXY` 时，`/health`、`/v1/provider` 和状态轮询可能被送往代理 |
| 修复 | 已完成 | `smoke-provider-federation.sh` 启动前补齐大小写两套 `NO_PROXY/no_proxy`，保留调用方既有值并追加 `127.0.0.1,localhost` |
| 防回归 | 已完成 | provider smoke unit 锁定两套变量；在无 bypass + 无效本地代理环境下实际运行 provider smoke 仍通过 |
| 验证 | 已通过 | provider smoke、quick coverage、release gate unit、含 provider federation 的完整 release gate 全部通过 |

### 2026-07-13 其余本地 smoke/预览代理隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 多个直接启动本地 Gateway 或 Python 预览的脚本只依赖继承环境；单独执行时，宿主 `ALL_PROXY/HTTP_PROXY` 可能接管本地 curl 探针 |
| 修复 | 已完成 | auth、CLI、resident、shell dual/direct、start-terminal、restart-gateway、start-web-preview 在入口统一追加大小写两套 `NO_PROXY/no_proxy`；公网 ingress 保留外部代理语义 |
| 防回归 | 已完成 | 对应脚本单测锁定两套变量；所有五类 Gateway smoke 与 web preview 在无 bypass + 无效代理环境中实测通过 |
| 验证 | 已通过 | 8 个脚本单测、bash 语法检查、相关 smoke、quick coverage 与含 provider federation 的完整 release gate 均通过 |

### 2026-07-13 provider/mirror 写接口 Bearer 门禁闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 生产模式关闭 dev bypass 时，provider connect/disconnect 与 world mirror source 写请求可在缺失 Bearer 下继续执行到业务层 |
| 修复 | 已完成 | `http_router.rs` 新增统一 `dispatch_admin_write`，三条高风险配置写路由先执行 `require_admin_auth`；开发/测试 fixture 的 `LOBSTER_DEV_AUTH_BYPASS=1` 行为保持不变 |
| 防回归 | 已完成 | 新增 Gateway 测试覆盖三条路由缺失 Bearer 必须返回 401；现有 provider federation roundtrip 继续覆盖 dev fixture 路径 |
| 验证 | 已通过 | TDD 红灯→绿灯、Gateway 290、workspace、fmt/clippy，以及无效本地代理环境下含 provider federation 的完整 release gate 全部通过 |

### 2026-07-13 Export Bearer 身份绑定闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 生产模式关闭 dev bypass 时，仅携带 `resident_id` query 的 `/v1/export` 请求仍可返回 200，未验证调用者身份 |
| 修复 | 已完成 | Gateway 在导出前要求 Bearer 会话且校验 `resident_id` 与会话身份一致；CLI export 增加 `--token` / session cache 认证 GET |
| 防回归 | 已完成 | Gateway 覆盖缺失、伪造、不匹配和匹配会话四条路径；CLI parser 与无 token login 提示测试锁定使用方式 |
| 验证 | 已通过 | TDD 红灯→绿灯、Gateway 291、CLI 116 unit + 22 integration、workspace、fmt/clippy 与完整 release gate 均通过 |

### 2026-07-13 CLI 写操作认证闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现并收口 | 生产模式下 `lobster-cli send` 已有 sidecar token，但后续 `edit/recall` 仍无认证，无法完成 agent 的发送→编辑→撤回闭环 |
| 合同 | 已锁定 | CLI edit/recall 透传 `actor_address=user:<id>` / `agent:<id>`；Gateway 校验 typed address 与 legacy `actor` 一致，并按 user session 或 agent sidecar token 鉴权；旧浏览器请求省略该字段时保持原 session 路径 |
| 防回归 | 已完成 | 新增 CLI 参数/请求模型测试、Gateway 无 token 401 与有效 agent token 200 测试；`smoke-cli-channel.sh` 继续覆盖 edit/recall 状态投影 |
| 真实验证 | 已通过 | 生产配置 `LOBSTER_DEV_AUTH_BYPASS=0` + `LOBSTER_AGENT_TOKENS=agent:openclaw=...` 下，CLI send/edit/recall 全部成功；workspace、release gate、fmt/clippy 全绿 |

### 2026-07-13 CLI scoped GET Bearer parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 关闭 dev bypass 时，`/v1/cli/inbox`、`/v1/cli/rooms`、`/v1/cli/tail` 缺失 Bearer 仍可返回 200，scoped read 未与写接口保持同一身份边界 |
| 修复 | 已完成 | Gateway 三条 scoped GET 统一调用 `require_cli_sender_auth`；CLI `inbox/rooms/tail` 支持 user session `--token` 与 agent sidecar `--agent-token`，请求使用带 Bearer 的 GET |
| 防回归 | 已完成 | Gateway 覆盖缺失/有效 sidecar token；CLI integration 覆盖无 token 时给出 login 提示；CLI channel、resident mainline、terminal smoke 均取得真实 session 后执行所有 scoped reads |
| 验证 | 已通过 | TDD 红灯→绿灯；Gateway 292、CLI 117 unit + 23 integration、workspace、web-shell 1310、fmt/clippy 与含 provider federation 的 release gate 均通过 |

### 2026-07-13 CLI read/presence Bearer parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | CLI `read` / `presence` 原先直接调用未认证 `post_json`；无 token 时会把请求送到 Gateway，错误只在网络/服务端阶段暴露 |
| 修复 | 已完成 | `run_read` / `run_presence` 统一解析 user session 并调用 `auth::post_json_authenticated`；agent sidecar token 对 resident state 返回明确不支持错误；Gateway 原有 `require_authenticated_actor` 保持不变 |
| 防回归 | 已完成 | CLI integration 覆盖 read/presence 无 token login 提示；CLI channel smoke 使用真实 session 覆盖 presence + mark-read 成功响应；新增 helper 单测锁定 agent 拒绝语义 |
| 验证 | 已通过 | TDD 红灯→绿灯；Gateway 292、CLI 117 unit + 23 integration、CLI smoke、workspace、fmt/clippy 与含 provider federation 的 release gate 均通过 |

### 2026-07-13 CLI admin read Bearer parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | Gateway 已对 /v1/admin/config、/v1/admin/residents、/v1/admin/rooms 要求 admin Bearer，但 CLI 仍用未认证 run_query，无 token 时会直接尝试网络请求 |
| 修复 | 已完成 | CLI admin 只读统一解析 user session，使用 auth::get_authenticated；agent sidecar token 明确拒绝；--token/登录缓存合同已同步帮助与使用手册 |
| 防回归 | 已完成 | CLI integration 覆盖三条命令无 token 的 login 提示；CLI channel smoke 以真实 session 覆盖 config/residents/rooms-admin JSON 读取；helper 单测锁定 agent 拒绝语义 |
| 验证 | 已通过 | TDD 红灯→绿灯；CLI 聚焦测试、CLI smoke、workspace、fmt/clippy 与 release gate 继续保持全绿 |

### 2026-07-13 CLI scoped search Bearer parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | CLI scoped read auth 回归先将 `/v1/cli/search` 纳入缺失 Bearer 测试；路由尚未存在时返回 404，不能提供统一 401 门禁 |
| 修复 | 已完成 | 新增受保护的 `/v1/cli/search`；Gateway 按 user/agent identity 过滤可见 direct/public conversations，显式不可见 `room_id` 拒绝；CLI search 要求 `--for` 并透传 session/sidecar Bearer |
| 兼容边界 | 已锁定 | 当时旧 H5 `/v1/shell/messages/search` 保持不变以避免触碰冻结的 `apps/lobster-web-shell/app.js`；CLI 不再调用该全局端点；后续认证收口见 2026-07-27 最新区块 |
| 防回归 | 已完成 | Gateway 覆盖缺失/有效 Bearer 与私聊越权搜索；CLI integration 覆盖无 token login 提示；CLI channel smoke 覆盖文本/JSON 搜索结果 |
| 验证 | 已通过 | Gateway 293、CLI 119 unit + 24 integration、workspace、CLI smoke、web-shell 1310、release gate（含 provider federation）、fmt/clippy 与 diff 检查全部通过 |

### 2026-07-13 场景编辑器清除语义闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现并收口 | admin-ds 选择默认场景或删除全部热点时发送 `null`，Gateway 原先把 `null` 与“字段未提供”混为一谈，旧自定义层无法清除 |
| 合同 | 已锁定 | `image_layer` / `hotspot_layer` 使用三态 patch：省略=保持、`null`=清除、对象=替换；同时覆盖 shell owner 更新与 admin 更新 |
| 防回归 | 已完成 | 新增 HTTP 场景自定义→`null` 清除测试，并锁定 admin-ds 透传清除 payload；保留旧字段省略兼容性 |
| 验证 | 已通过 | 聚焦 Gateway/admin-ds 测试、全量 workspace/release gate、fmt/clippy 均通过 |

### 2026-07-13 admin-ds HTTP 失败反馈闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现并收口 | 设备添加、权限组创建只判断网络 `error`；Gateway 返回 HTTP 失败时 `{ok:false}` 会落入成功分支，造成假成功提示 |
| 修复 | 已完成 | 两处 await 写操作统一改为 `error` → `ok` → HTTP 状态的三段判定，失败不清空表单、不刷新成功数据 |
| 防回归 | 已完成 | 新增 admin-ds 静态测试锁定 `/v1/admin/devices/add` 与 `/v1/admin/permission-groups` 必须显式检查 `ok` |
| 验证 | 已通过 | Web-shell 1299 tests、layout、realness、workspace/release gate 全绿 |

### 2026-07-13 admin-ds 次级数据真实空态闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | Gateway 连接后邀请码读取失败、审计接口返回空数组或失败时，旧状态仍保留 `admin-ds-data.js` 的本地 mock |
| 修复 | 已完成 | Gateway 正式路径对邀请码/审计日志统一执行空态清理；成功空数组保留空数组，失败响应显示错误提示；仅无 Gateway 本地预览继续使用 mock |
| 防回归 | 已完成 | 新增 2 个 admin-ds runtime 测试，覆盖 Gateway 空响应和 5xx 失败，锁定不回退旧邀请码/日志 |
| 验证 | 已通过 | Web-shell 1305 tests、layout、realness 全绿 |

### 2026-07-13 admin-ds 主投影真实空态闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | `loadGatewayAdminData()` 只在 normalized 数组非空时覆盖居民、房间、消息；Gateway 返回空数组或 HTTP `null` 时旧本地 mock 继续展示 |
| 修复 | 已完成 | 主投影按响应合同逐项覆盖：有效空数组写入空态，失败/畸形响应清空对应数组；部分失败仪表盘明确显示“部分同步”，不再标记成本地预览 |
| 防回归 | 已完成 | 新增 2 个 admin-ds runtime 测试，覆盖空投影、503 失败、空态数组和部分同步提示 |
| 验证 | 已通过 | Web-shell 1305 tests、layout、realness 全绿 |

### 2026-07-13 admin-ds 权限组真实空态闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | Gateway 连接后 `/v1/admin/permission-groups` 返回空数组或失败时，渲染器仍回退四类本地内置权限组，造成未持久化角色的假投影 |
| 修复 | 已完成 | Gateway 路径严格显示正式权限组；空数组显示 Gateway 空态，失败清空旧组并提示；无 Gateway 本地预览继续保留内置说明 |
| 防回归 | 已完成 | 新增运行时测试覆盖 Gateway 空组和离线内置组两条路径 |
| 验证 | 已通过 | Web-shell 1305 tests、layout、realness 全绿 |

### 2026-07-13 admin-ds 房间加入规则真实来源闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 权限与邀请卡片硬编码四个房间的开放/邀请/白名单状态，但当前 Gateway 没有对应加入规则字段或读取端点 |
| 修复 | 已完成 | 删除静态策略投影，显示“Gateway 合同未提供房间加入规则”的明确待接入空态；后续合同落地后再接正式数据 |
| 防回归 | 已完成 | 静态测试锁定不可用标记、根因文案，并禁止四个硬编码房间规则重新出现 |
| 验证 | 已通过 | Web-shell 1306 tests、layout、realness、workspace、fmt/clippy 与 release gate 全绿；provider federation 本轮显式跳过 |

### 2026-07-13 admin-ds 世界公告与安全治理真实空态闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 世界公告和安全治理读取成功后再遇到 5xx/畸形响应时，旧公告、通告、举报和制裁列表仍保留 |
| 修复 | 已完成 | Gateway 失败/畸形响应统一清空四类投影并渲染空态；无 Gateway 时也不保留伪造治理数据 |
| 防回归 | 已完成 | 新增运行时测试覆盖公告失败和安全三列表失败清空；有效空数组仍保留正式空态 |
| 验证 | 已通过 | Web-shell 1308 tests、layout、realness、workspace、fmt/clippy 与 release gate 全绿；provider federation 本轮显式跳过 |

### 2026-07-13 admin-ds 系统服务健康真实来源闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 系统配置页硬编码 Gateway、SMTP、AI、配额、存储和 Webhook 状态，但 Gateway 没有对应服务健康明细合同 |
| 修复 | 已完成 | 删除静态服务健康样例，改为明确“Gateway 合同未提供服务健康明细”的待接入空态；Gateway 实时配置编辑器保持不变 |
| 防回归 | 已完成 | 静态测试锁定不可用标记、根因文案，并禁止公网端点/模型/配额等样例重新出现 |
| 验证 | 已通过 | Web-shell 1309 tests、layout、realness、release gate、node --check 与 diff check 全绿；workspace 基线仍全绿 |

### 2026-07-13 admin-ds 仪表盘事件真实来源闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 未连接 Gateway 时仪表盘直接显示 resident_demo、公网 IP、AI 通道等硬编码审计事件；这些数据不在 mock 工厂或 Gateway 合同中 |
| 修复 | 已完成 | 静态事件改为空态；Gateway 成功同步后仍由 `renderDashboardEvents(auditPayload)` 使用正式审计投影 |
| 防回归 | 已完成 | 静态测试锁定不可用标记和“Gateway 未连接”根因文案，并禁止旧身份/IP/模型样例重新出现 |
| 验证 | 已通过 | Web-shell 1310 tests、layout、realness、release gate、node --check 与 diff check 全绿；workspace 基线仍全绿 |

### 2026-07-13 smoke 运行时输入契约闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | release gate 会把环境中的 provider-only `BIN_PATH` 当作普通 Gateway 默认路径，可能在 `SKIP_BUILD=1` 下误用 release artifact 或错误二进制 |
| 修复 | 已完成 | 普通 smoke 固定使用 `GATEWAY_BIN`/`CLI_BIN`/`TUI_BIN` 覆盖或 debug 默认；provider smoke 独立使用 `GATEWAY_ARTIFACT`/`BIN_PATH`，release gate 不再导出 `BIN_PATH` |
| 防回归 | 已完成 | 新增 `test_smoke_runtime_contract_unit.py`，覆盖 shell smoke、web dual-browser、provider artifact 与 release gate 的构建/二进制/产物顺序，并纳入 quick coverage 与 release gate |
| 验证 | 已通过 | runtime contract unit、脚本单测、`bash -n`、完整 release gate 全绿；`SKIP_BUILD=1` 下 provider federation 按配置跳过 |

### 2026-07-13 TUI Gateway Bearer parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | TUI `gateway_post`、`gateway_get` 和 `/v1/direct/open` 原先不带 Bearer；关闭 dev bypass 后编辑、撤回、管理命令和私聊打开会被 Gateway 拒绝 |
| 修复 | 已完成 | 统一读取 `LOBSTER_SESSION_TOKEN`（优先）或 `LOBSTER_AGENT_TOKEN`，为 TUI Gateway 控制请求设置 `Authorization: Bearer ...`；无 token 时保留本地 dev/只读兼容路径 |
| 防回归 | 已完成 | 新增 TUI Bearer header 归一化测试；resident mainline smoke 在两条 TUI 启动路径显式透传已验证 session token |

### 2026-07-13 TUI scoped search parity 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | CLI/Gateway 已有受保护的 scoped search，但 TUI 没有对应命令；直接复用冻结的 H5 全局搜索会绕过当前会话范围 |
| 修复 | 已完成 | TUI 增加 `/search <关键词>`，以当前 active conversation 的 `room_id`、当前 launch identity 调用 `/v1/cli/search`，沿用 session/agent Bearer；结果投影为终端通知 |
| 兼容边界 | 已锁定 | 当时 `/search` 不修改 `apps/lobster-web-shell/app.js`，旧 H5 搜索保持原合同；后续 H5 认证收口已同步修改搜索控制器接线，见 2026-07-27 最新区块 |
| 防回归 | 已完成 | 新增 URL 编码/身份与房间范围测试、Gateway 命中结果投影测试，并覆盖 `/help` 文案 |
| 验证 | 已通过 | TUI 230 tests、workspace、fmt/clippy、resident mainline smoke 与含 provider federation 的完整 release gate 全部通过；web-shell 1310 tests 及 layout/realness 也保持通过 |

### 2026-07-13 TUI scoped search resident smoke 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | resident mainline smoke 虽然覆盖 TUI `/dm`，但没有真实执行 `/search`；原有脚本模式执行后也不会输出脚本期间的终端通知，无法验证搜索分支 |
| 修复 | 已完成 | TUI smoke script 在脚本结束后支持 `LOBSTER_TUI_SMOKE_DUMP=plain/json` 投影；resident smoke 追加 `/search $DM_TEXT` 并捕获输出，断言当前私聊报告命中 |
| 兼容边界 | 已锁定 | 只增强测试/烟测执行路径，不改变 H5 合同，不修改 `apps/lobster-web-shell/app.js` |
| 防回归 | 已完成 | resident smoke unit 锁定命令、输出捕获和命中断言；保留既有 session token、动态端口和清理合同 |
| 验证 | 已通过 | 脚本单测、`bash -n`、TUI 230 tests、clippy、resident mainline smoke 与含 provider federation 的完整 release gate 全部通过 |

### 2026-07-13 web-shell smoke 完整测试集闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | `scripts/smoke-web-shell.sh` 使用 Node `--test-force-exit` 时虽返回 0，但只报告 1280 tests，低于 npm unit 的 1310，发布门禁存在静默漏测 |
| 修复 | 已完成 | 移除 `--test-force-exit`，让 Node test runner 自然等待所有异步测试完成；不改变页面代码或测试内容 |
| 防回归 | 已完成 | smoke unit 锁定无 force-exit；直接执行 smoke 报告 1310 passed，`npm test` 的 1310 unit、layout、realness 继续通过 |
| 验证 | 已通过 | `scripts/smoke-web-shell.sh` 报告 1310 passed；`npm test` 的 1310 unit、layout、realness 通过；含 provider federation 的完整 release gate 通过 |

### 2026-07-13 Gateway 生产启动空时间线闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已确认 | `GatewayRuntime::open()` 在任何全新 state dir 都会写入 `system` / `builder` / `dm:builder:rsaga` 三条 demo 消息；生产默认配置也会把它们暴露给 H5/导出读取 |
| 修复 | 已完成 | demo seed 现在只在测试构建或显式 `LOBSTER_DEV_AUTH_BYPASS=1` 的本地 fixture 中启用；生产默认 `LOBSTER_DEV_AUTH_BYPASS=0` 从空 timeline 启动，既有真实写入路径不变 |
| 防回归 | 已完成 | 新增 `demo_messages_seed_only_for_tests_or_explicit_dev_bypass`，覆盖生产 false、显式 dev bypass 和测试构建三种决策；实际 release binary 分别用 bypass=0/1 启动验证空/有 seed |
| 兼容边界 | 已锁定 | 不改 H5、TUI、CLI 或 `apps/lobster-web-shell/app.js`；现有 smoke fixture 显式设置 bypass，测试构建继续保留既有 seed 依赖 |
| 验证 | 已通过 | Gateway 294 tests、Gateway clippy、fmt；生产 binary bypass=0 返回 0 条消息，bypass=1 返回 demo lobby 消息；移动端 realness 与双浏览器 smoke 继续通过 |

### 2026-07-13 resident-scoped shell state/SSE Bearer 绑定闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 生产模式关闭 dev bypass 时，`/v1/shell/state?resident_id=alice` 与 `/v1/shell/events?resident_id=alice` 原先不校验 Bearer，调用者可伪造 query identity；admin-ds 的 resident-scoped 读取没有服务端身份绑定 |
| 修复 | 已完成 | read route 在解析 `resident_id` 后统一复用 `require_authenticated_actor`；Bearer 缺失、无效或与 query 身份不一致均返回 401；无 `resident_id` 的匿名公共 shell projection 合同保持不变 |
| 防回归 | 已完成 | 新增 HTTP 回归覆盖 state/SSE 缺失 token、匹配 token、错配 token 三类路径，并断言匹配 SSE 仍返回 shell-state 事件 |
| 兼容边界 | 已锁定 | admin-ds 已通过 `fetchGatewayJson` 透传 session token；未改 `apps/lobster-web-shell/app.js`，未改变无 resident scope 的公共 H5 初始化路径 |
| 验证 | 已通过 | TDD 红灯→绿灯；Gateway focused test 通过，fmt 通过；后续完整 workspace/release gate 复验中 |

### 2026-07-13 provider federation smoke 鉴权契约闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | provider federation smoke 使用合成 `smoke-bot` 通过下游发送消息，但双 Gateway 默认关闭 dev bypass，消息请求被真实 Bearer 门禁返回 401 |
| 修复 | 已完成 | 仅在上下游 smoke fixture 启动命令前显式设置 `LOBSTER_DEV_AUTH_BYPASS=1`；不改变 Gateway 默认生产鉴权行为 |
| 防回归 | 已完成 | `test_smoke_provider_federation_unit.py` 锁定两个 Gateway 启动都必须显式带 bypass；保留 artifact、上下游桥接和清理契约 |
| 验证 | 已通过 | provider federation smoke 通过；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 SKIP_BUILD=1` 完整 release gate 通过，provider interlink smoke included |

### 2026-07-13 provider federation smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | smoke 默认固定使用 `18787/18788`，已有 Gateway 或并行运行会导致上下游启动/健康检查互相冲突 |
| 修复 | 已完成 | 默认端口改为空值并由 `python3` 预留两个不同的本机临时端口；显式传入端口仍按调用方配置执行，并拒绝上下游相同端口 |
| 防回归 | 已完成 | provider smoke unit 锁定自动预留逻辑、两个端口分支、`python3` 依赖与原有 artifact/清理合同 |
| 验证 | 已通过 | provider federation smoke 自动端口运行通过；含 provider 的完整 release gate 通过（本次实际使用动态端口） |

### 2026-07-13 resident mainline smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | resident mainline smoke 默认固定使用 `8800`，两个 smoke 并行运行时会争用 Gateway 端口 |
| 修复 | 已完成 | 默认 `PORT` 改为空值并由 `python3` 预留本机临时端口；显式 `PORT` 仍可复现/固定指定端口 |
| 防回归 | 已完成 | resident smoke unit 锁定动态端口函数与空值分支；保留现有 Gateway/CLI/TUI binary、session token 和清理合同 |
| 验证 | 已通过 | 单次 resident smoke 通过；两个未指定端口的 resident smoke 并行运行均通过，自动分配不同端口 |

### 2026-07-13 CLI channel smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | CLI channel smoke 默认固定使用 `8796`，并行运行会争用 Gateway，导致 send/edit/recall 主链路互相干扰 |
| 修复 | 已完成 | 默认 `PORT` 改为空值并由 `python3` 预留本机临时端口；显式 `PORT` 仍可固定指定端口 |
| 防回归 | 已完成 | CLI smoke unit 锁定动态端口函数与空值分支；保留 sidecar token、send/edit/recall、tail follow 和重启恢复合同 |
| 验证 | 已通过 | 单次 CLI channel smoke 通过；两个未指定端口的 CLI smoke 并行运行均通过，自动分配不同端口 |

### 2026-07-13 auth registration smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | auth registration smoke 默认固定使用 `8799`，并行 OTP/黑名单流程会争用 Gateway 端口 |
| 修复 | 已完成 | 默认 `PORT` 改为空值并由 `python3` 预留本机临时端口；显式 `PORT` 仍可固定指定端口 |
| 防回归 | 已完成 | auth smoke unit 锁定动态端口函数与空值分支；保留 OTP 注册、持久化状态和黑名单拒绝合同 |
| 验证 | 已通过 | 单次 auth smoke 通过；两个未指定端口的 auth smoke 并行运行均通过，自动分配不同端口 |

### 2026-07-13 shell dual HTTP smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | shell dual HTTP smoke 默认固定使用 `8807`，并行运行会争用 Gateway 端口 |
| 修复 | 已完成 | 默认 `PORT` 改为空值并由 `python3` 预留本机临时端口；显式 `PORT` 仍可固定指定端口 |
| 防回归 | 已完成 | shell dual HTTP smoke unit 锁定动态端口函数与空值分支；保留 shell state/events、SSE 和 delivered message 合同 |
| 验证 | 已通过 | 单次 shell dual HTTP smoke 通过；两个未指定端口的 shell dual smoke 并行运行均通过，自动分配不同端口 |

### 2026-07-13 shell direct HTTP smoke 端口隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | shell direct HTTP smoke 默认固定使用 `8808`，并行运行会争用 Gateway 端口 |
| 修复 | 已完成 | 默认 `PORT` 改为空值并由 `python3` 预留本机临时端口；显式 `PORT` 仍可固定指定端口 |
| 防回归 | 已完成 | shell direct HTTP smoke unit 锁定动态端口函数与空值分支；保留 direct open、成员读权限、send/edit/recall、SSE 和 outsider 禁止访问合同 |
| 验证 | 已通过 | 单次 shell direct HTTP smoke 通过；两个未指定端口的 shell direct smoke 并行运行均通过，自动分配不同端口 |

### 2026-07-13 start-terminal 外部 Gateway 契约闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 设置 `LOBSTER_WAKU_GATEWAY_URL` 指向不可达地址时，旧脚本进入本地 Gateway 启动分支，却仍等待覆盖后的外部 URL，最终超时且误启动本地进程 |
| 修复 | 已完成 | 增加 `GATEWAY_URL_OVERRIDE`；显式 URL 只走外部复用路径，不可达立即返回明确错误；未设置覆盖时保持默认本地 `HOST:PORT` 自动启动 |
| 防回归 | 已完成 | start-terminal shell unit 锁定覆盖变量、fail-fast 分支和本地 `nohup` 顺序；保留 TTY、state/log、TUI mode 与 Gateway URL 透传合同 |
| 验证 | 已通过 | 外部 URL 不可达行为测试立即退出并输出“配置 Gateway 不可达”；`test_start_terminal.py` 在本地代理绕过后通过，相关 unit 与 `bash -n` 通过 |

### 2026-07-13 terminal smoke 本地代理隔离闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | 宿主设置 `ALL_PROXY` 且未设置本地 bypass 时，`test_start_terminal.py` 的 curl 健康探针走代理，Gateway 实际已监听仍被判定为超时 |
| 修复 | 已完成 | `local_probe_env()` 为 curl 子进程合并保留原代理配置，同时强制追加 `127.0.0.1,localhost` 到大小写两套 bypass 变量 |
| 防回归 | 已完成 | `test_start_terminal_unit.py` 新增环境透传断言，锁定两套变量均包含本地地址 |
| 验证 | 已通过 | 单元测试 4 项通过；不额外设置 `NO_PROXY` 直接运行 terminal smoke 通过；含 provider federation 的完整 release gate 复验通过 |

### 2026-07-13 TUI `/dm` Gateway fail-closed 闭环

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯 | 已复现 | TUI 设置 `LOBSTER_WAKU_GATEWAY_URL` 后，`/v1/direct/open` 401/网络失败会静默回退本地 canonical direct ID，造成未确权私聊和假成功状态 |
| 修复 | 已完成 | 抽出 `resolve_direct_conversation_id()`：无 Gateway 时返回离线 fallback；有 Gateway 时严格透传 URL/响应错误，不创建本地直聊 |
| 防回归 | 已完成 | 新增注入式 Gateway 失败测试，锁定 `/v1/direct/open` URL 和错误透传；既有无 Gateway `/dm` 离线兼容测试保留 |
| 验证 | 已通过 | `cargo fmt --all -- --check`、`cargo test -p lobster-tui`：227 passed / 0 failed |

## 2026-07-07 进度收口: 6-26~28 WIP 提交 + 继续减债

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 6-26~28 WIP 提交 | 已完成 | commit 298758e：私宅访问确权+好友关系流+前端纯状态下沉（35 文件 +3628/-284），此前停留工作树未提交 |
| 测试基线复验 | 已通过 | Gateway 274 passed/0 fail/0 warn；web-shell 1185 tests/0 fail（unit+layout+realness） |
| 主题实验隔离 | 已保留 | neon-pixel 主题/mockup/private-room-alt01 素材未接入页面，留作 untracked，不入主线提交 |

### 当前真实进度

| 模块 | 估算 | 说明 |
| --- | ---: | --- |
| P0 单城 IM 闭环 | 99% | 私宅主客访问确权+好友关系流已提交，剩余仅上线环境复验 |
| P1 空间交互 | 82% | 场景编辑器/移动端仍有 polish 空间 |
| P2 后台运维 | 93% | admin-ds 设备/场景/写操作护栏已接入 |
| P3 技术债 | 74% | app.js 9342 行（目标<8700），最大剩余债 |
| P4 TUI/CLI parity | 95% | 后续以 release smoke 复验为主 |
| P5 跨城/加密 | 15% | 后置 |

### 下一步

继续 app.js TDD 减债（9342→<8700）：优先抽 userDetailCard 投影纯模型、renderRoomStagePortrait 内联计算、shellMode 视图状态等低风险纯函数，每次配套 node --test。

## 2026-07-07 续: app.js TDD 减债推进

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| userDetailCard 投影下沉 | 已完成 | commit 22d60b9：6 内联函数 → shell-user-detail-card.js *ForState 注入式纯函数，+15 单测，app.js -65 行 |
| conversationCallout 文案下沉 | 已完成 | commit ef93132：3 内联模型 → shell-conversation-callout.js *ForState，+10 单测，app.js -53 行 |
| 消息动作 payload 下沉 | 已完成 | gatewayMessagePayload/editMessage/recallMessage payload → shell-message-action-payload.js *ForState，+5 单测，app.js -2 行 |
| app.js 累计 | 9342→9222 | 三轮减债 -120 行；npm test 1215 全绿（unit + layout + realness） |
| 交付完整性 | 已完成 | README 进度章节更新到 7-07（Gateway 274/Web 1185）；CHANGELOG.md 创建；CI 覆盖三端确认 |
| 端到端真实 smoke | 已通过 | `SKIP_BUILD=1 node scripts/smoke-web-dual-browser.mjs`：真实 gateway+双浏览器验证消息发送/编辑/撤回/失败重发闭环（503 注入重发测试通过） |

### 下一步

剩余 app.js 候选：消息动作 payload/guard（小）、shellMode 视图状态文案（小）、Quick-action 读取器（大但循环依赖复杂，需谨慎评估）。app.js 大量剩余是 DOM 编排+全局状态管理，纯函数拆分边际递减；P3 目标 <8700 行在 6-26~28 新增私宅/好友功能后需重新评估合理性。建议转向端到端 smoke 复验与上线环境真实可用性验证（P0 最后一公里）。

## 2026-06-28 P3 技术债推进: 直聊打开请求状态纯模型下沉

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 降债 | 已完成 | `openDirectSession()` 不再内联 trim / 空居民 / 自聊校验和 `/v1/direct/open` payload 拼装，改为消费 `directSessionOpenRequestState()` |
| Gateway 合同 | 保持不变 | 实际 `POST /v1/direct/open`、表单 reset、focus room、刷新 Gateway 投影仍由 `app.js` 编排，H5 不新增私聊会话真值 |
| 请求状态 | 已覆盖 | `directSessionOpenRequestState()` 统一产出 offline / empty-peer / self / allowed 请求状态，并保留既有私聊打开与就绪文案 |
| 防回归测试 | 已完成 | `shell-governance-render.test.mjs` 覆盖 direct open 状态；`shell-pages-static.test.mjs` 锁定 app.js 通过 helper 消费直聊请求模型 |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-governance-render.test.mjs --test-name-pattern "directSessionOpenRequestState"
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "direct session open"
node --test apps/lobster-web-shell/test/fake-dom-import-rewrite.test.mjs
```

## 2026-06-28 P3 技术债推进: 好友关系提交状态纯模型下沉

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 降债 | 已完成 | `submitResidentRelationshipAction()` 不再直接判断 `model.endpoint` / `model.payload` 和无网关状态，改为消费 `residentRelationshipSubmitRequestState()` |
| Gateway 合同 | 保持不变 | 实际 `POST /v1/resident-relationships/request|accept`、Bearer session、刷新 Gateway 投影仍由 `app.js` 编排，H5 不新增好友关系真值 |
| 提交状态 | 已覆盖 | `residentRelationshipSubmitRequestState()` 统一产出 noop / offline / allowed 请求状态，并保留申请、接受、默认更新成功文案 |
| 防回归测试 | 已完成 | `shell-governance-render.test.mjs` 覆盖提交状态；`shell-pages-static.test.mjs` 锁定 app.js 通过 helper 消费关系提交模型 |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-governance-render.test.mjs --test-name-pattern "residentRelationship"
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "resident relationship"
node --test apps/lobster-web-shell/test/fake-dom-import-rewrite.test.mjs
```

## 2026-06-28 P3 技术债推进: 私宅策略提交闸门纯状态下沉

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 降债 | 已完成 | `submitPersonalRoomAccessPolicy()` 不再内联判断 policy 集合、房主权限和响应 policy 兜底，改为消费 `shell-personal-room-policy.js` 的纯状态 helper |
| Gateway 合同 | 保持不变 | 实际 `POST /v1/personal-room/access-policy`、Bearer session、刷新 Gateway 投影仍由 `app.js` 编排，H5 不新增私有权限真值 |
| 提交闸门 | 已覆盖 | `personalRoomAccessPolicySubmitRequestState()` 统一产出 invalid-policy / not-owner / offline / allowed 请求状态，保留既有状态文案 |
| 响应兜底 | 已覆盖 | `appliedPersonalRoomAccessPolicy()` 只接受 Gateway 返回的合法 policy；异常响应回退到请求 policy，再回退保守默认 `friends_only` |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-personal-room-policy.test.mjs
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "personal room access policy"
node --check apps/lobster-web-shell/app.js && node --check apps/lobster-web-shell/shell-personal-room-policy.js
```

## 2026-06-28 P3 技术债推进: 治理状态条纯状态下沉

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 降债 | 已完成 | 新增 `apps/lobster-web-shell/shell-governance-status.js`，把治理提示条文案前缀、错误状态和动态 class 清理规则从 `app.js` 下沉为纯 helper |
| 交互行为 | 保持不变 | `app.js` 继续只负责选择 `#governance-status` / `worldStateEl` 并应用 text/class，不改变好友关系、私宅访问或 Gateway 写路径 |
| fake-dom 映射 | 已同步 | `test/fake-dom.mjs` 已加入新模块，避免 app.js 本地 import 在 fake-dom 运行时遗漏重写 |
| 防回归测试 | 已完成 | 新增 `shell-governance-status.test.mjs` 覆盖 user/非 user 文案、fallback 文案、错误 class 与私宅访问提示 class；静态测试锁定 app.js 通过模块消费 |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-governance-status.test.mjs
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "resident relationship"
node --test apps/lobster-web-shell/test/fake-dom-import-rewrite.test.mjs
```

## 2026-06-27 P3 技术债推进: 私宅访问策略控件纯状态下沉

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 降债 | 已完成 | 新增 `apps/lobster-web-shell/shell-personal-room-policy.js`，把私宅访问策略默认值、标签和控件状态计算从 `app.js` 下沉为纯 helper |
| 控件行为 | 保持不变 | `app.js` 继续负责 DOM 应用与 Gateway `POST /v1/personal-room/access-policy`，不新增 H5 私有权限真值 |
| fake-dom 映射 | 已同步 | `test/fake-dom.mjs` 已加入新模块，防止 app.js 本地 import 改动破坏运行时测试 |
| 防回归测试 | 已完成 | 新增 `shell-personal-room-policy.test.mjs` 覆盖 owner/visitor/offline/saving/online 状态；`shell-pages-static.test.mjs` 锁定 app.js 通过模块消费 |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-personal-room-policy.test.mjs
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "personal room access policy"
node --test apps/lobster-web-shell/test/fake-dom-import-rewrite.test.mjs
```

## 2026-06-27 P2 收口: admin-ds 设备管理 UI 主内容接入

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 设备管理模块结构 | 已修复 | `mod-devices` 原本位于 `.ds-content` 结束后、右侧详情面板之后；现已移动到后台主内容区，跟随 `data-module="devices"` 正常模块切换 |
| 设备管理控件 | 已覆盖 | 静态测试将 `deviceAddressInput` / `deviceLabelInput` / `deviceAddBtn` / `deviceTableBody` 纳入后台结构合同 |
| Gateway 写操作 | 已复核 | 继续复用既有 `/v1/admin/devices/add|remove|block|unblock` 和 `GET /v1/admin/devices`，不新增 H5 私有状态 |
| 防回归测试 | 已完成 | `admin-ds-static.test.mjs` 新增主内容区层级测试，防止设备模块再次漂移到后台布局外 |

### 验证

```bash
node --test apps/lobster-web-shell/test/admin-ds-runtime.test.mjs apps/lobster-web-shell/test/admin-ds-static.test.mjs
```

## 2026-06-27 P2 收口: 私宅关系按钮移动端验收与未授权提示

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 住宅页关系反馈 | 已修复 | `creative.html` 新增 `#governance-status`，好友申请/接受与私宅访问拦截不再无处显示 |
| 未授权私宅提示 | 已增强 | `residentPrivateRoomAccessPromptModel()` 返回的 `resident-room-access-note` / `is-locked` / `is-pending` / `is-actionable` class 现在会落到状态条，形成可识别的空态反馈 |
| 移动端关系按钮 | 已验收 | `.creative-resident-list .resident-relationship-action` 最小高度提升到 34px；realness 在 390px 移动视口验证按钮可点击且未被覆盖 |
| 防回归测试 | 已完成 | `shell-pages-static.test.mjs` 锁定状态节点、class 传递与 CSS；`verify-frontend-realness.mjs` 增加 mobile relationship actions 检查 |

### 验证

```bash
node --test apps/lobster-web-shell/test/shell-pages-static.test.mjs --test-name-pattern "resident relationship"
node apps/lobster-web-shell/verify-frontend-realness.mjs
```

## 2026-06-27 P0 收口: world-square / admin-ds 注册登录 JS 接线

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 共享登录接线 | 已完成 | 新增 `apps/lobster-web-shell/shell-auth-standalone.js`，复用 `shell-auth.js` 的 OTP 流程，统一处理 `loadAuthDraft()` / `persistAuthDraft()` / `requestEmailOtp()` / `verifyEmailOtp()` / `updateAuthFormState()` |
| 世界广场登录 | 已接入 | `world-square.html` 改为调用 `initStandaloneAuthSurface()`；登录成功后继续刷新右上角“登录/连线中”状态 |
| admin-ds 登录 | 已接入 | `admin-ds.html` 不再手写 `initAuth` 细节，改为共享 standalone 登录模块，避免后台页复制认证逻辑 |
| 可选字段兼容 | 已修复 | `shell-auth.js` 的邮箱/手机 OTP 请求兼容没有 `auth-mobile-input` / `auth-device-input` 的页面；空的反滥用字段不再写入 payload |
| 防回归测试 | 已完成 | `shell-auth.test.mjs`、`admin-ds-static.test.mjs`、`shell-pages-static.test.mjs` 已锁定共享接线和可选字段行为 |

### 验证

```bash
node --test test/shell-auth.test.mjs test/admin-ds-static.test.mjs test/shell-pages-static.test.mjs
npm test
node --check shell-auth-standalone.js
```

## 2026-06-27 P0 复验: `?gateway=` 前端真实消息发送闭环

### 复验结论

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 双浏览器真实发送 | 已验证 | `scripts/smoke-web-dual-browser.mjs` 启动真实 Gateway + 静态 Web 服务，分别打开 `index.html?gateway=...&identity=qa-a` 和 `creative.html?gateway=...&identity=qa-b` |
| 消息闭环 | 已验证 | qa-a 与 qa-b 可跨页面互看 self/peer 投影；覆盖发送、编辑、撤回 |
| 失败重发 | 已验证 | smoke 故意让一次 `/v1/shell/message` 返回 503，确认 H5 显示失败 pending 气泡并可重发，最终 peer 端收到提交后的消息 |
| 当前门禁位置 | 已确认 | `make smoke-e2e` 会执行真实 Playwright 双浏览器 smoke；`smoke-release-gate.sh` 仅跑该脚本的 quick unit，避免发布快速门禁强制启动浏览器 |

### 验证

```bash
SKIP_BUILD=1 node scripts/smoke-web-dual-browser.mjs
```

## 2026-06-27 P1 收口: admin-ds 场景编辑器 day/night URL 输入

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 正式场景编辑器 | 已接入 | `admin-ds` 的“场景编辑”模块现在和房间详情面板一致，支持白天背景图 URL 与夜晚背景图 URL 输入 |
| Gateway payload | 已接入 | 保存场景时 `image_layer` 会携带 `day_image_url` / `night_image_url`；只填自定义图片、不选 preset 时也会提交 `preset: custom` |
| 成对约束 | 复用 Gateway | 前端显示“白天+夜晚必须成对填写”，最终校验仍由 Gateway `SceneImageLayer` 合同负责，避免 H5 私有权限/素材真值 |
| 防回归测试 | 已完成 | `admin-ds-static.test.mjs` 切入 `renderSceneEditor()` 函数体，防止只靠房间详情旧面板误判 |

### 验证

```bash
node --test test/admin-ds-runtime.test.mjs test/admin-ds-static.test.mjs
npm test
git diff --check
```

## 2026-06-26 产品确权: 私宅主客访问权限边界

### 新增蓝图约束

| 项目 | 结论 |
| --- | --- |
| 主客访问前提 | 用户必须是已注册、已登录的 IM 居民，未登录访客不可进入任何居民私宅 |
| 房主确权 | 是否允许他人访问由房主自己设置，不允许前端或 Gateway 默认把 `home:<resident>` 当作全公开房间 |
| MVP 策略 | 至少支持 `registered_all`（所有已登录注册用户）和 `friends_only`（好友/互相关系）两档 |
| 默认策略 | 未配置时采用保守默认；不自动等同于所有注册用户可访问。`friends_only` 仅对 Gateway 已确认的好友关系放行 |
| 消息隔离 | 私宅场景展示与私聊消息流必须分层；允许进入场景不代表允许读取历史私聊消息 |
| CC 必读 | 详见 `docs/DEVELOPMENT_BLUEPRINT.md` 的“私宅主客访问确权（2026-06-26）” |

### 对当前 WIP 的影响

本轮已把 Gateway 默认行为收口为登录 + 房主策略确权：

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 私宅识别 | 已收口 | 只有 `conversation_id == home:<owner>` 且参与者正好是 `<owner>` 的 1 人 Direct 才被视为 personal room；兼容旧 `dm:<id>` 半锚定 Direct |
| 创建权限 | 已收口 | `open_personal_room()` 要求房主是已注册居民；`POST /v1/personal-room` 要求 Bearer token 与 `resident_id` 匹配 |
| 默认可见性 | 已收口 | 未登录/匿名 shell state 不暴露私宅；默认 `friends_only` 下其他注册居民不能看到房主私宅；房主本人可见 |
| 策略表 | 已完成后端合同 | 新增 `registered_all` / `friends_only` access policy，持久化到 `personal-room-access-policies.json`，并在个人房间 shell state 暴露 `personal_room_access_policy` |
| 策略设置端点 | 已完成后端合同 | `POST /v1/personal-room/access-policy` 要求 Bearer token 与房主 `resident_id` 匹配 |
| H5 房主策略控件 | 已接入 | 住宅页仅在“自己的私宅”显示 `好友` / `注册` 分段控件；提交复用现有 Bearer session，Gateway 仍负责最终房主校验 |
| 好友关系模型 | 已完成后端合同 | 新增 `request` / `accept` 两步关系流，持久化到 `resident-relationships.json`；pending 不解锁，accepted friends 才能访问 `friends_only` 私宅场景 |
| 居民目录关系投影 | 已完成 Gateway 合同 | `GET /v1/residents?resident_id=<viewer>` 会按访问者投影 `relationship_state` / `relationship_requested_by`；H5 不需要本地伪造好友状态 |
| H5 关系入口 | 已接入 | H5 加载世界状态时会用带 `resident_id` 的居民目录覆盖 snapshot 居民列表；常规居民目录和住宅侧栏均显示 `申请好友` / `已申请` / `接受好友` / `好友`，提交复用 Bearer session |
| H5 未授权私宅提示 | 已接入 | 点击未授权的 `personal_room_id` 时不再切到不可见 room；按登录/申请/等待/接受状态提示用户下一步，并保留 Gateway 作为唯一权限真源 |
| 防消息泄漏 | 已收口 | `registered_all` 只开放私宅场景可见性；非房主访客看到 room 时不携带私宅历史消息 |
| 后续体验提示 | 待完善 | 继续做真实移动端验收、按钮触控 polish 和更完整的空态视觉；仍必须复用 Gateway 关系端点，不在 H5 本地伪造好友状态 |

CC/DS 后续不要再把 1 人 Direct 默认视为全公开主页；如要继续开放访问，必须复用 Gateway access policy 合同和测试，不要在 H5 私自放行。

## 2026-06-20 Codex 技术债推进: Rust 生产 panic 扫描门禁固化

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯契约 | 完成 | `test_scripts_quick_unit_coverage.py` 与 `test_smoke_release_gate_unit.py` 先要求新增 Rust 生产 panic 扫描，并确认缺脚本/缺挂载会失败 |
| 扫描脚本 | 完成 | 新增 `scripts/rust-production-panic-scan.py`，覆盖 Gateway / CLI / TUI / crates 生产 Rust 源，排除测试文件与 `#[cfg(test)]` item |
| 崩溃宏防回归 | 完成 | 扫描器除 `.unwrap()` / `.expect()` / `panic!()` 外，也拦截生产 `todo!()` / `unimplemented!()` / `unreachable!()`，防止占位实现进入运行路径 |
| 假阳性处理 | 完成 | 扫描器在计算作用域前剥离字符串字面量，避免 `"\n}\n"`、`format!("{x}")` 等测试字符串打乱花括号计数 |
| 扫描器自验证 | 完成 | `test_rust_production_panic_scan_unit.py` 通过临时 Rust fixture 验证生产 `.unwrap()` 与 `unimplemented!()` 会失败、`#[cfg(test)]` 中 `.unwrap()` 会被忽略，带内部引号/跨行内容的 raw string 与块注释不会误报 |
| 字符串/注释过滤 | 完成 | 扫描匹配前剥离普通字符串、可跨行 Rust raw string、`//` 行注释与 `/* ... */` 块注释，避免帮助文案或注释里的 `.unwrap(`/`panic!` 造成假红灯 |
| 缺失路径假绿防护 | 完成 | 显式 `--scan-root` 或默认扫描根缺失时直接失败并输出 `scan root missing`，避免目录移动/拼写错误让门禁静默通过 |
| Rust fmt 门禁 | 完成 | `smoke-release-gate.sh` 的非 `SKIP_BUILD` 路径先跑 `cargo fmt --check` 再跑 clippy/build；`verify-complete.sh` 也在 workspace test 后、lint 前挂载 `rust fmt` |
| Release gate | 完成 | `smoke-release-gate.sh` 先跑扫描器 quick unit，再跑真实 `rust-production-panic-scan.py`，避免只测脚本、不扫真实仓库 |
| 完整验证 | 完成 | `verify-complete.sh` 也拆成扫描器 quick unit + 真实生产扫描，并用 stub 单测锁定 PASS/FAIL 记账，避免完整验证漏掉真实扫描 |

### 验证

```bash
python3 scripts/test_rust_production_panic_scan_unit.py
python3 scripts/rust-production-panic-scan.py
python3 scripts/test_scripts_quick_unit_coverage.py
python3 scripts/test_smoke_release_gate_unit.py
python3 scripts/test_verify_complete_unit.py
bash -n scripts/verify-complete.sh
bash -n scripts/smoke-release-gate.sh
```

## 2026-06-19 Codex 技术债推进: verify-complete 假绿风险收口

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯契约 | 完成 | 新增 `scripts/test_verify_complete_unit.py`，锁定 `verify-complete.sh` 必须开启 `set -euo pipefail`，并用 `${PIPESTATUS[0]}` 读取 `cmd | tee` 中真实命令退出码 |
| 验收脚本 | 完成 | `verify-complete.sh` 改为 `run_logged()` 统一记录 PASS/FAIL，任一阶段失败都会让最终脚本退出非零，同时继续写完整日志 |
| 门禁覆盖 | 完成 | `test_scripts_quick_unit_coverage.py` 将 `verify-complete.sh` 纳入脚本 quick unit 映射；`smoke-release-gate.sh` 挂载 `complete verification unit`，只跑快速合同检查，不执行完整长耗时验收 |
| 真实失败路径 | 完成 | 用临时 stub 让 `npm test` 返回 7，验证脚本最终退出 `1`，日志包含 `FAIL: frontend`，后续成功项仍能继续记录 |
| 行为测试补强 | 完成 | `test_verify_complete_unit.py` 现在会实际执行 `verify-complete.sh` 的 stub 环境，覆盖 `npm` 失败与 `git status` 失败两条路径；工作区状态也改为 `run_logged "workspace status"`，不再绕过统一退出码 |
| crypto-mls panic 收口 | 完成 | `generate_key()` / `derive_epoch_key()` 从 `expect("RNG")` / `expect("HKDF")` / `expect("fill")` 改为 `Result` 错误传播；新增测试护栏防止生产 crypto helper 重新引入这些 panic |
| Gateway 安全头 panic 收口 | 完成 | `security_headers()` 不再对静态安全响应头 `unwrap()`；新增 `http_support` 单测锁定无 panic 构造并确认 `X-Content-Type-Options` / `X-Frame-Options` / `Referrer-Policy` 仍输出 |
| CORS 配置注入防护 | 完成 | `LOBSTER_CORS_ORIGIN` 若为空或包含控制字符（如换行注入）会回退 `*`；`cors_origin_reads_from_env` 覆盖合法、空值和非法换行配置，避免服务输出危险 header value |
| CORS 非 ASCII 配置防护 | 完成 | `LOBSTER_CORS_ORIGIN` 若包含非 ASCII 字符会回退 `*`；新增 `cors_origin_non_ascii_env_falls_back_to_wildcard`，避免环境变量误填导致 gateway 在 header 构造处 panic |
| Admin device 路由锁中毒防护 | 完成 | `http_device_routes.rs` 新增统一 `with_runtime()` 锁助手；设备列表/add/remove/block/unblock 遇到 poisoned runtime mutex 时返回 JSON 500，不再 `expect("poisoned")` 打崩请求线程 |
| Auth 路由锁中毒防护 | 完成 | `http_auth_routes.rs` 新增统一 `with_runtime()` 锁助手；session/preflight/email OTP/mobile OTP/logout 遇到 poisoned runtime mutex 时返回 JSON 500，不再 `expect("gateway runtime mutex poisoned")` 打崩请求线程 |
| City 写路由锁中毒防护 | 完成 | `require_admin_auth()` / `require_capability_or_bypass()` 遇到 poisoned runtime mutex 时返回 JSON 500；`http_city_write_routes.rs` 新增统一 `with_runtime()`，create/join/approve/steward/federation/public-room/freeze 不再依赖 `expect("gateway runtime mutex poisoned")` |
| Governance 写路由锁中毒防护 | 完成 | `http_governance_write_routes.rs` 新增统一 `with_runtime()`；world notice / city trust / safety report / review / advisory / sanction / unsanction 遇到 poisoned runtime mutex 时返回 JSON 500，并保留 bearer actor 校验和 unsanction 审计写入 |
| Read 路由锁中毒防护 | 完成 | `http_read_routes.rs` 统一走 `with_runtime()`；provider/shell/world/admin/read-only CLI 入口遇到 poisoned runtime mutex 时返回 JSON 500，不再依赖 `expect("gateway runtime mutex poisoned")`；`export` 也移除 `resident_id validated above` 的生产 `expect` |
| Write 路由锁中毒防护 | 完成 | `http_write_routes.rs` 新增统一 `with_runtime()`；provider/direct/waku/shell message/scene/edit/recall/presence/mark-read/CLI/admin 写入口遇到 poisoned runtime mutex 时返回 JSON 500，不再依赖 `expect("gateway runtime mutex...")`；业务变更与审计写入仍保留在同一 runtime 作用域 |
| Gateway main/notifier 锁中毒防护 | 完成 | `main.rs` 启动期 upstream 状态打印不再因 runtime mutex poisoned panic；`GatewayStateNotifier` 的 mutex/condvar poisoned 后恢复 inner guard，SSE generation/notify/wait 路径不再依赖 `expect("gateway notifier...")` |
| Admin actor unwrap 收口 | 完成 | `http_write_routes.rs` 的 ban/unban/freeze/unfreeze/config/moderate admin actor 校验改为 `required_admin_actor()` 显式返回 401，不再保留 `actor.unwrap()` 生产路径 |
| Runtime 时间 helper panic 收口 | 完成 | `GatewayRuntime::now_ms()` 不再 `expect("system time should be after unix epoch")`；系统时间异常时回退 `0`，极端未来时间 clamp 到 `i64::MAX` |
| Gateway query parser 覆盖补强 | 完成 | 新增 `split_path_and_query_keeps_unescaped_query_components_intact`，补上普通未转义 query key/value 不被截断的回归覆盖，和既有 percent escape 测试形成完整边界 |
| Gateway 静态 header helper panic 收口 | 完成 | `json_header()` / `text_header()` / `sse_header()` / `no_cache_header()` / `cors_*_header()` 改为返回 `Option<Header>`；响应构造统一用 `ResponseHeaderExt::with_optional_header()`，header 构造失败时跳过该 header 而不是 panic |
| Gateway 生产 panic 扫描 | 完成 | 排除 `gateway_tests.rs` / `gateway_test_support.rs` 后，Gateway 生产文件在 `#[cfg(test)]` 前的 `.expect()` / `.unwrap()` / `panic!()` 扫描为空 |
| Rust workspace 基线复验 | 完成 | `cargo test --workspace` 通过；覆盖 CLI 100 unit + 18 gateway integration + 5 integration、TUI 225 unit、Gateway 264 unit、核心 crates 与 doc-tests |
| Complete verification 覆盖补强 | 完成 | `verify-complete.sh` 新增 `cargo test --workspace` 与 `cargo clippy --workspace -- -D warnings`，避免“完整验证”只覆盖 gateway/cli/tui 三个包而漏掉核心 crates/doc-tests/lint |
| Release gate lint 挂载 | 完成 | `smoke-release-gate.sh` 在非 `SKIP_BUILD=1` 的发布路径中先跑 `cargo clippy --manifest-path "$ROOT_DIR/Cargo.toml" --workspace -- -D warnings`，再 build 三个共享 debug binary |

### 验证

```bash
python3 scripts/test_verify_complete_unit.py
python3 scripts/test_scripts_quick_unit_coverage.py
python3 scripts/test_smoke_release_gate_unit.py
bash -n scripts/verify-complete.sh scripts/smoke-release-gate.sh
make lint
python3 scripts/test_package_release_unit.py && python3 scripts/test_scripts_quick_unit_coverage.py && python3 scripts/test_smoke_release_gate_unit.py && python3 scripts/test_smoke_provider_federation_unit.py && python3 scripts/test_smoke_web_dual_browser_unit.py && python3 scripts/test_smoke_resident_mainline_unit.py && python3 scripts/test_smoke_cli_channel_unit.py && python3 scripts/test_smoke_auth_registration_unit.py && python3 scripts/test_smoke_shell_dual_http_unit.py && python3 scripts/test_smoke_shell_direct_http_unit.py && python3 scripts/test_smoke_web_shell_unit.py && python3 scripts/test_install_server_unit.py && python3 scripts/test_preview_server_unit.py && python3 scripts/test_start_terminal_shell_unit.py && python3 scripts/test_audit_web_assets_unit.py && python3 scripts/test_lobster_device_id_unit.py && python3 scripts/test_start_web_preview_unit.py && python3 scripts/test_restart_gateway_unit.py && python3 scripts/test_preflight_unit.py && python3 scripts/test_smoke_public_ingress_unit.py && python3 scripts/test_smoke_install_layout_unit.py && python3 scripts/test_start_terminal_unit.py && python3 scripts/test_makefile_unit.py && python3 scripts/test_verify_complete_unit.py
bash -n scripts/package-release.sh scripts/smoke-provider-federation.sh scripts/smoke-release-gate.sh scripts/smoke-resident-mainline.sh scripts/smoke-cli-channel.sh scripts/smoke-auth-registration.sh scripts/smoke-shell-dual-http.sh scripts/smoke-shell-direct-http.sh scripts/install-server.sh scripts/smoke-web-shell.sh scripts/start-terminal.sh scripts/audit-web-assets.sh scripts/lobster-device-id.sh scripts/restart-gateway.sh scripts/preflight.sh scripts/smoke-public-ingress.sh scripts/smoke-install-layout.sh scripts/verify-complete.sh
zsh -n scripts/start-web-preview.sh
node --check scripts/preview-server.mjs
node --check scripts/smoke-web-dual-browser.mjs
cargo test -p lobster-waku-gateway cors_origin_non_ascii_env_falls_back_to_wildcard
cargo test -p lobster-waku-gateway admin_devices_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway auth_session_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway create_city_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway city_write_routes_do_not_depend_on_runtime_lock_expect
cargo test -p lobster-waku-gateway publish_world_notice_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway governance_write_routes_do_not_depend_on_runtime_lock_expect
cargo test -p lobster-waku-gateway provider_status_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway read_routes_do_not_depend_on_runtime_lock_expect
cargo test -p lobster-waku-gateway provider_disconnect_returns_500_when_runtime_lock_poisoned
cargo test -p lobster-waku-gateway write_routes_do_not_depend_on_runtime_lock_expect
cargo test -p lobster-waku-gateway gateway_main_does_not_depend_on_runtime_lock_expect
cargo test -p lobster-waku-gateway gateway_notifier_recovers_from_poisoned_mutex
cargo test -p lobster-waku-gateway gateway_notifier_does_not_depend_on_poison_expect
cargo test -p lobster-waku-gateway write_routes_do_not_depend_on_actor_unwrap
cargo test -p lobster-waku-gateway core_runtime_now_ms_does_not_depend_on_system_time_expect
cargo test -p lobster-waku-gateway split_path_and_query_keeps_unescaped_query_components_intact
cargo test -p lobster-waku-gateway static_header_helpers_do_not_depend_on_panic_paths
cargo test -p crypto-mls
cargo test -p lobster-waku-gateway
cargo test --workspace
cargo fmt --check
for f in apps/lobster-waku-gateway/src/*.rs; do case "$f" in */gateway_tests.rs|*/gateway_test_support.rs) continue ;; esac; hits=$(sed '/^#\[cfg(test)\]/,$d' "$f" | rg -n "\.expect\(|\.unwrap\(\)|panic!\(" || true); if [[ -n "$hits" ]]; then printf '%s\n%s\n' "-- $f" "$hits"; fi; done
git diff --check -- apps/lobster-waku-gateway/src/http_auth_routes.rs apps/lobster-waku-gateway/src/http_city_write_routes.rs apps/lobster-waku-gateway/src/http_device_routes.rs apps/lobster-waku-gateway/src/http_governance_write_routes.rs apps/lobster-waku-gateway/src/http_support.rs apps/lobster-waku-gateway/src/http_write_routes.rs apps/lobster-waku-gateway/src/gateway_tests.rs crates/crypto-mls/src/lib.rs scripts/verify-complete.sh scripts/test_verify_complete_unit.py scripts/test_scripts_quick_unit_coverage.py scripts/test_smoke_release_gate_unit.py scripts/smoke-release-gate.sh docs/ACTIVE_WORK_QUEUE.md
```

## 2026-06-09 Codex 技术债校准与修复

### Web Shell realness 回归修复

| 项目 | 结果 |
|------|------|
| 红灯 | `npm test` 的 `verify-frontend-realness.mjs` 失败：`/unified.html rail width should stay on the shared 220px token`，实测 `.world-entry-rail` 被后加载的 `.sfc-layout` 通用规则压成 160px |
| 修复 | 调整 `unified.html` 样式加载顺序，让 `styles.creative.css` 先加载，`styles.world-entry.css` 后加载；world-entry 专属 220px rail token 重新成为最终级联结果 |
| 验证 | `npm test` 通过：736 unit passed / 0 failed，layout passed，realness passed |

### Gateway unsanction 安全/审计债收口

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `unsanction_resident_endpoint_records_actor_audit_event`，确认解除制裁没有写 `admin:unsanction_resident` 审计事件 |
| 红灯 | 新增 `unsanction_resident_endpoint_rejects_oversized_body`，确认超 1MiB 请求体没有走统一 size limit |
| 修复 | `/v1/admin/residents/unsanction` 改用 `read_request_body()`；校验 `actor_id` / `sanction_id`；可选 bearer token 与 actor 匹配；解除成功后写审计并 `notify_changed()` |
| 验证 | `LOBSTER_DEV_AUTH_BYPASS=1 cargo test -p lobster-waku-gateway -- --test-threads=1` 通过：244 passed / 0 failed；此前 3 个 warning 清零 |

### CLI export parity 补齐

| 项目 | 结果 |
|------|------|
| 基线 | `cargo test -p lobster-tui` 通过：217 passed；`cargo test -p lobster-cli` 通过：31 unit + 11 gateway integration + 5 integration passed |
| 红灯 | 新增 `export_command_parses_gateway_export_request`，确认 CLI 不支持 `export`：`unsupported command: export` |
| 修复 | CLI 新增 `export --for user:... [--conversation-id ...] [--format md/jsonl/txt] [--include-public] [--json]`，对接 Gateway `/v1/export`；默认人类输出直接打印 `content`，JSON 模式输出完整响应 |
| 防回归 | 新增 `export_command_rejects_room_actor_target` 和 `export_command_prints_export_content_by_default`，防止把 room 当居民导出、或吞掉导出正文 |
| 验证 | `cargo test -p lobster-cli` 通过：34 unit + 11 gateway integration + 5 integration passed |

### Workspace 级测试 auth 环境债收口

| 项目 | 结果 |
|------|------|
| 红灯 | `cargo test --workspace` 失败：Gateway 244 个测试中 22 个 admin/city 写接口用例返回 401；根因是测试默认依赖外部 `LOBSTER_DEV_AUTH_BYPASS=1`，workspace 命令没有注入该环境变量 |
| 修复 | `GatewayRuntime` 新增实例级 `dev_auth_bypass`；测试构建默认开启，生产构建仍默认关闭；`require_admin_auth()` / capability gate 改为读取 runtime 实例，避免全局 env 污染并行测试 |
| 防回归 | `resident_without_capability_is_denied_admin_action` 改为关闭当前 runtime 的 test bypass，并携带 Bearer header 验证 capability 拒绝；不再 `remove_var/set_var` 影响其他测试 |
| 验证 | `cargo test -p lobster-waku-gateway` 通过：244 passed / 0 failed；`cargo test --workspace` 通过：全部 Rust workspace unit/integration/doc tests 绿 |

### 根目录 npm test 入口债收口

| 项目 | 结果 |
|------|------|
| 红灯 | 在仓库根目录执行 `npm test` 失败：`Missing script: "test"`；但多处文档和协作提示会把 `npm test` 当作前端验收入口，容易误导 CC/DS 在错误目录得出假失败 |
| 修复 | 根 `package.json` 新增 `test` / `test:frontend`，统一代理到 `apps/lobster-web-shell`；同步 root `package-lock.json` 元数据 |
| 验证 | 仓库根目录 `npm test` 通过：代理执行 web-shell 736 unit passed / 0 failed，layout passed，realness passed |

### Makefile Gateway 测试入口 workaround 收口

| 项目 | 结果 |
|------|------|
| 债务 | `make test-gateway` 仍保留旧 workaround：`LOBSTER_DEV_AUTH_BYPASS=1 cargo test -p lobster-waku-gateway -- --test-threads=1`；这会掩盖 Gateway 测试是否真的能在默认并行环境下通过 |
| 修复 | `Makefile` 的 `test-gateway` 改回标准 `cargo test -p lobster-waku-gateway`，与已修复的实例级 test bypass 保持一致 |
| 验证 | `make test-gateway` 通过：244 passed / 0 failed；`Makefile` 中已无 `LOBSTER_DEV_AUTH_BYPASS` / `--test-threads=1` |

### Clippy lint 入口收口

| 项目 | 结果 |
|------|------|
| 红灯 | `make lint` 失败：`core_runtime.rs` 的 `map_or(true, ...)` 触发 `clippy::unnecessary-map-or`；`http_support.rs` 的 `request.as_reader().bytes()` 触发 `clippy::unbuffered-bytes` |
| 修复 | 搜索过滤改用 `Option::is_none_or()`；请求体读取改为一次 `take(MAX_BODY_SIZE + 1).read_to_end()`，超过 1MiB 时按原语义报错，不再额外逐字节读取 |
| 验证 | `make lint` 通过；`unsanction_resident_endpoint_rejects_oversized_body` 与 `message_search_finds_text_in_conversation` 均通过 |

### Rust fmt 入口收口

| 项目 | 结果 |
|------|------|
| 红灯 | `cargo fmt --check` 失败，多个 Rust 文件存在格式化差异，主要来自近期 Gateway / CLI / TUI / crypto-mls 改动 |
| 修复 | 执行 `cargo fmt` 做机械格式化，不做语义修改 |
| 验证 | `cargo fmt --check`、`make lint`、`make check`、`make test` 全部通过 |

### Smoke 门禁基线复验

| 项目 | 结果 |
|------|------|
| 范围 | `make smoke` 覆盖 shell 双 HTTP 冒烟与 web-shell 冒烟 |
| 结果 | `smoke-shell-dual-http.sh` 临时启动 Gateway `127.0.0.1:8807`，`qa-a` 发送公共 shell 消息后 `qa-b` 成功收到；随后 `smoke-web-shell.sh` 跑完 web-shell 测试套件 |
| 验证 | `make smoke` 通过，当前技术债收口后的集成 smoke 基线为绿 |

### TUI edit/recall Gateway parity 补齐

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `edit_command_without_args_shows_usage_without_publishing_plain_text` / `recall_command_without_args_shows_usage_without_publishing_plain_text`，确认 TUI 会把 `/edit`、`/recall` 空命令误发布成普通正文 |
| 合同 | 新增 `terminal_edit_command_request_matches_gateway_shell_contract` / `terminal_recall_command_request_matches_gateway_shell_contract`，锁定 TUI 调用 Gateway 的 `/v1/shell/message/edit` 与 `/v1/shell/message/recall` payload |
| 修复 | TUI 终端新增 `/edit <消息ID> <新正文>` 与 `/recall <消息ID>`；帮助文案同步暴露；命令失败时写终端 notice，不再走普通消息发布路径 |
| 防回归 | 新增带参数命令测试，确认 `/edit msg text` / `/recall msg` 在 Gateway 未配置时写失败 notice 且不会落入本地普通消息 publish；`/help` 同步断言列出 edit/recall |
| 可测性 | `handle_terminal_submission_with_gateway_post()` 让 edit/recall 成功路径可通过注入 POST 函数测试，不再需要改全局 `LOBSTER_WAKU_GATEWAY_URL`；新增 success notice 测试覆盖 edit/recall 成功响应 |
| 验证 | 聚焦红灯测试转绿；`cargo test -p lobster-tui` 通过：225 passed / 0 failed；`make check`、`cargo fmt --check` 与 `make lint` 通过 |

### CLI edit/recall smoke parity 补齐

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_cli_channel_unit.py`，先确认 `smoke-cli-channel.sh` 没有 edit/recall 步骤；`test_smoke_release_gate_unit.py` 同步要求 release gate 挂载该脚本单测 |
| 修复 | `smoke-cli-channel.sh` 在 JSON 模式后新增 edit/recall smoke：先 `send --json` 获取两条 message_id，再分别执行 `edit --actor ... --conversation-id ... --message-id ... --json` 与 `recall --actor ... --conversation-id ... --message-id ... --json` |
| 防回归 | 脚本继续通过 `tail --json` 校验编辑后消息 `is_edited=true` 且正文为新文本，撤回后消息 `is_recalled=true` 且正文投影为 `消息已撤回`；release gate 在真实 CLI channel smoke 前先跑脚本合同单测 |
| 验证 | `python3 scripts/test_smoke_cli_channel_unit.py`、`python3 scripts/test_smoke_release_gate_unit.py`、`bash -n scripts/smoke-cli-channel.sh scripts/smoke-release-gate.sh` 通过；`SKIP_BUILD=1 scripts/smoke-cli-channel.sh` 通过 |

### make smoke 门禁覆盖面补齐

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_makefile_unit.py`，确认 `make smoke` 仍只声明并执行 shell/web smoke，没有纳入 CLI channel smoke |
| 修复 | `Makefile` 的 `make smoke` 帮助文案改为 `CLI + shell + web smoke`；实际执行顺序新增 `python3 ./scripts/test_smoke_cli_channel_unit.py` 与 `./scripts/smoke-cli-channel.sh`，再跑原 shell dual HTTP 与 web-shell smoke |
| 验证 | `python3 scripts/test_makefile_unit.py`、CLI/release smoke 单测、`bash -n` 通过；`make smoke` 通过，真实覆盖 CLI send/inbox/rooms/tail/follow/edit/recall、shell dual HTTP、web-shell 736 tests |

### Release gate smoke 合同漂移收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 先要求 release gate 挂载 `test_makefile_unit.py`；随后 `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=0 SKIP_BUILD=1 scripts/smoke-release-gate.sh` 暴露 resident mainline 红灯：匿名 `/v1/cities/join` 现在先被 admin bearer gate 拒绝 |
| 修复 | `smoke-release-gate.sh` 在 CLI smoke 单测前新增 `makefile smoke unit`；`smoke-resident-mainline.sh` 改为先完成 OTP 注册拿真实 `session_token`，再用 Bearer header 校验 unregistered join 的业务错误，并用同一 Bearer 执行 registered join |
| 验证 | `SKIP_BUILD=1 scripts/smoke-resident-mainline.sh` 通过；`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=0 SKIP_BUILD=1 scripts/smoke-release-gate.sh` 通过，覆盖 CLI/auth/resident/shell dual/shell direct/web/terminal smoke，provider federation 本轮显式跳过 |

### Terminal smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 要求 release gate 在长耗时 `test_start_terminal.py` 前运行 `test_start_terminal_unit.py`，先确认该 quick unit 未被挂载 |
| 修复 | `smoke-release-gate.sh` 在 `terminal smoke` 前新增 `terminal smoke unit`，让 terminal smoke 的 Python helper 合同先快速失败 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_start_terminal_unit.py`、`bash -n scripts/smoke-release-gate.sh` 通过 |

### Provider federation smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_provider_federation_unit.py` 锁定 provider federation smoke 的 artifact 解包、release build、上下游 Gateway、`remote-gateway` 状态、下游发消息上游可见与清理逻辑；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在可选的 `provider federation smoke` 前新增 `provider federation smoke unit`，让 provider interlink 脚本合同先快速失败 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_provider_federation_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-provider-federation.sh` 通过 |

### Install server quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 先要求 release gate 在 install layout quick unit 前挂载 `scripts/test_install_server_unit.py`，确认 `install-server.sh` 本体缺少直接合同检查 |
| 修复 | 新增 `scripts/test_install_server_unit.py` 锁定 `install-server.sh` 的安装路径默认值、host target/artifact 校验、Rust bootstrap、systemd unit、nginx site、冲突 Gateway 清理、端口占用检查与 health/provider 探针合同；`smoke-release-gate.sh` 在 `install layout smoke unit` 前新增 `install server unit` |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_install_server_unit.py`、`python3 scripts/test_smoke_install_layout_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/install-server.sh scripts/smoke-install-layout.sh` 通过 |

### Install layout smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_install_layout_unit.py` 锁定 install layout smoke 的假 systemctl/nginx/curl、artifact 生成、`install-server.sh` 调用、systemd/nginx 产物与 health/provider 探针；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `install layout smoke unit`；只跑快速脚本合同检查，完整 `smoke-install-layout.sh` 仍按发布文档单独执行 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_install_layout_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-install-layout.sh` 通过 |

### Public ingress smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_public_ingress_unit.py` 锁定 public ingress smoke 的 `BASE_URL` 输入、首页标记、GET/HEAD `/health`、`/v1/provider` 与临时文件清理合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `public ingress smoke unit`；只跑快速脚本合同检查，真实外部入口 smoke 仍需按 `BASE_URL=... ./scripts/smoke-public-ingress.sh` 单独执行 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_public_ingress_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-public-ingress.sh` 通过 |

### Package release quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_package_release_unit.py` 锁定 `package-release.sh` 的 dist 目录、host target、release build、source/web/gateway artifact、排除目录与缺失 gateway binary warning 合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `package release unit`；只跑快速脚本合同检查，真实 `package-release.sh` 仍按发布流程单独执行 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_package_release_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/package-release.sh` 通过 |

### Preflight quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_preflight_unit.py` 锁定 `preflight.sh` 的命令依赖、target triple、内存/磁盘探测、cargo 1.85 edition-2024 floor、Linux/systemd/nginx 提示合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在真实 preflight 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在真实 `preflight` 前新增 `preflight unit`；即使 `RUN_PREFLIGHT=0` 也会保留脚本合同快速检查 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_preflight_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/preflight.sh` 通过 |

### Restart gateway quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_restart_gateway_unit.py` 锁定 `restart-gateway.sh` 的 debug gateway 构建、端口旧进程发现/停止、nohup 启动、日志路径、`/health`、shell state 与 admin summary 探针合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `restart gateway unit`；只跑快速脚本合同检查，不执行真实本地 gateway 重启 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_restart_gateway_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/restart-gateway.sh` 通过 |

### Web preview quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_start_web_preview_unit.py` 锁定 `start-web-preview.sh` 的 zsh 入口、默认端口/根目录、PID/log 文件、已有 preview 复用、非 preview 端口拒绝、python http.server 启动与 readiness 检查合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `web preview unit`；只跑快速脚本合同检查，不启动真实本地预览服务 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_start_web_preview_unit.py`、`bash -n scripts/smoke-release-gate.sh`、`zsh -n scripts/start-web-preview.sh` 通过 |

### Device id quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_lobster_device_id_unit.py` 锁定 `lobster-device-id.sh` 的网卡优先级、platform UUID fallback、默认 URL、已有 query 参数拼接与纯 MAC 输出合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `device id unit`；只跑快速脚本合同检查，不读取真实网卡/UUID |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_lobster_device_id_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/lobster-device-id.sh` 通过 |

### Web assets audit quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_audit_web_assets_unit.py` 锁定 `audit-web-assets.sh` 的 assets 目录检查、图片类型枚举、最大图片排序、256px 派生图引用统计、source/concepts 大文件扫描合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `web assets audit unit`；只跑快速脚本合同检查，不执行真实 assets 扫描 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_audit_web_assets_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/audit-web-assets.sh` 通过 |

### Start terminal shell quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_start_terminal_shell_unit.py` 锁定 `start-terminal.sh` 的 TTY 防护、Gateway 复用/启动、state/log 目录、TUI build、`LOBSTER_WAKU_GATEWAY_URL` 传递与 `--mode` 参数合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在长 terminal smoke 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在 `terminal smoke` 前新增 `start terminal shell unit`；只跑快速脚本合同检查，不进入真实交互式 TUI |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_start_terminal_shell_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/start-terminal.sh` 通过 |

### Preview server quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_preview_server_unit.py` 锁定 `preview-server.mjs` 的默认 root/host/port、MIME 表、路径解析、路径穿越拒绝、目录 index、404 与 HEAD 响应合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在结束前新增 `preview server unit`；只跑快速脚本合同检查，不启动真实 Node 预览服务 |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_preview_server_unit.py`、`bash -n scripts/smoke-release-gate.sh`、`node --check scripts/preview-server.mjs` 通过 |

### Web dual browser smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_web_dual_browser_unit.py` 锁定 `smoke-web-dual-browser.mjs` 的 Gateway/Web 双进程、Playwright 双页面、index/creative 身份 URL、edit/recall、pending retry 与进程/state 清理合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在 terminal smoke 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在 `web shell smoke` 后新增 `web dual browser smoke unit`；只跑快速脚本合同检查，不启动真实 Playwright 双浏览器 smoke |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_web_dual_browser_unit.py`、`bash -n scripts/smoke-release-gate.sh`、`node --check scripts/smoke-web-dual-browser.mjs` 通过 |

### Auth registration smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_auth_registration_unit.py` 锁定 `smoke-auth-registration.sh` 的 inline dev OTP、auth preflight、email OTP request/verify、auth-state 持久化、world-safety sanction 与黑名单 OTP 拦截合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在真实 auth smoke 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在 `auth registration smoke` 前新增 `auth registration smoke unit`；只跑快速脚本合同检查，不启动真实 Gateway |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_auth_registration_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-auth-registration.sh` 通过 |

### Resident mainline smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 先要求 release gate 在真实 `resident mainline smoke` 前挂载 `scripts/test_smoke_resident_mainline_unit.py`，确认该 quick unit 缺失 |
| 修复 | 新增 `scripts/test_smoke_resident_mainline_unit.py` 锁定 `smoke-resident-mainline.sh` 的 inline OTP 注册、Bearer join、未注册居民业务错误、CLI rooms/tail、TUI direct/user 脚本与 cleanup 合同；`smoke-release-gate.sh` 在真实 resident mainline smoke 前新增 `resident mainline smoke unit` |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_resident_mainline_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-resident-mainline.sh` 通过 |

### Shell dual HTTP smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_shell_dual_http_unit.py` 锁定 `smoke-shell-dual-http.sh` 的 Gateway 启动、初始 shell state、SSE after 订阅、公共消息发送、peer state 可见与 cleanup 合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在真实 shell dual smoke 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在 `shell dual HTTP smoke` 前新增 `shell dual HTTP smoke unit`；只跑快速脚本合同检查，不启动真实 Gateway |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_shell_dual_http_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-shell-dual-http.sh` 通过 |

### Shell direct HTTP smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | 新增 `scripts/test_smoke_shell_direct_http_unit.py` 锁定 `smoke-shell-direct-http.sh` 的 direct open、参与者 projection、SSE after 订阅、direct 发送、edit/recall、outsider 读写拦截与 cleanup 合同；`test_smoke_release_gate_unit.py` 先确认 release gate 未在真实 shell direct smoke 前挂载该 quick unit |
| 修复 | `smoke-release-gate.sh` 在 `shell direct HTTP smoke` 前新增 `shell direct HTTP smoke unit`；只跑快速脚本合同检查，不启动真实 Gateway |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_shell_direct_http_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-shell-direct-http.sh` 通过 |

### Web shell smoke quick unit 挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 先要求 release gate 在真实 `web shell smoke` 前挂载 `scripts/test_smoke_web_shell_unit.py`，确认该 quick unit 缺失 |
| 修复 | 新增 `scripts/test_smoke_web_shell_unit.py` 锁定 `smoke-web-shell.sh` 从仓库根执行 `node --test --test-force-exit apps/lobster-web-shell/test/*.test.mjs`，且不依赖 root `npm test` 或 generated 目录；`smoke-release-gate.sh` 在真实 web shell smoke 前新增 `web shell smoke unit` |
| 验证 | `python3 scripts/test_smoke_release_gate_unit.py`、`python3 scripts/test_smoke_web_shell_unit.py`、`python3 scripts/test_smoke_web_dual_browser_unit.py`、`bash -n scripts/smoke-release-gate.sh scripts/smoke-web-shell.sh`、`node --check scripts/smoke-web-dual-browser.mjs` 通过 |

### Scripts quick unit coverage 护栏挂入 release gate

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 先要求 release gate 最前置挂载 `scripts/test_scripts_quick_unit_coverage.py`；新增覆盖率测试后，它自身也先红在 release gate 未包含该护栏 |
| 修复 | 新增 `scripts/test_scripts_quick_unit_coverage.py`，显式锁定 release/smoke/install/start 类脚本与对应 quick unit 的映射，并检查 release gate 已挂载关键 quick unit；`smoke-release-gate.sh` 在 preflight unit 前新增 `scripts quick unit coverage` |
| 验证 | `python3 scripts/test_scripts_quick_unit_coverage.py`、`python3 scripts/test_smoke_release_gate_unit.py`、`bash -n scripts/smoke-release-gate.sh` 通过 |

### SKIP_BUILD 预构建 smoke 依赖债收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_release_gate_unit.py` 与 CLI/auth/resident/shell HTTP quick unit 先要求 `need_cmd cargo` 必须位于 `SKIP_BUILD != 1` 构建分支内，确认现有脚本在跳过构建时仍无条件依赖 cargo |
| 修复 | `smoke-release-gate.sh`、`smoke-cli-channel.sh`、`smoke-auth-registration.sh`、`smoke-resident-mainline.sh`、`smoke-shell-dual-http.sh`、`smoke-shell-direct-http.sh` 改为仅在实际构建时检查 cargo；`SKIP_BUILD=1` 路径保留二进制存在性校验 |
| 验证 | 相关 quick unit、`bash -n` 通过；无 cargo 的 PATH 下执行 `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=0 SKIP_BUILD=1 ... bash scripts/smoke-release-gate.sh` 不再报 `missing command: cargo`，而是在真实 smoke 阶段按预期报缺预构建 gateway binary |

### Resident mainline 预构建二进制检查收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_resident_mainline_unit.py` 先要求 `smoke-resident-mainline.sh` 在创建 `STATE_ROOT` 前检查 `GATEWAY_BIN`、`CLI_BIN`、`TUI_BIN` 可执行，确认现有脚本会把缺二进制错误延迟到中段命令执行 |
| 修复 | `smoke-resident-mainline.sh` 在构建分支后新增 gateway/cli/tui 三个预构建二进制存在性检查；`SKIP_BUILD=1` 路径能提前给出明确错误，并避免创建临时 state |
| 验证 | `python3 scripts/test_smoke_resident_mainline_unit.py`、相关 release/smoke quick unit 与 `bash -n` 通过；无 cargo PATH 下分别构造缺 gateway、缺 cli、缺 tui，均提前返回对应 `binary not found` |

### Web dual browser 预构建 Gateway 检查收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_web_dual_browser_unit.py` 先要求 `smoke-web-dual-browser.mjs` 在创建 `stateRoot` 前检查 `GATEWAY_BIN` 可执行，并要求入口 catch 输出简洁错误信息 |
| 修复 | `smoke-web-dual-browser.mjs` 新增 `assertExecutable()`，在 `SKIP_BUILD=1`/构建后统一校验 gateway binary；入口失败输出改为优先打印 `error.message`，避免完整 stack/cause 污染 smoke 日志 |
| 验证 | `python3 scripts/test_smoke_web_dual_browser_unit.py`、相关 release/web/resident quick unit、`node --check scripts/smoke-web-dual-browser.mjs` 通过；缺 `GATEWAY_BIN` 时只输出 `gateway binary not found or not executable: ...`，且不会创建 `/tmp/lobster-web-dual-browser.*` state 目录 |

### Web dual browser Playwright 延迟加载收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_web_dual_browser_unit.py` 先要求 `smoke-web-dual-browser.mjs` 不再顶层静态 import Playwright，确认缺依赖会早于 `GATEWAY_BIN` 检查失败 |
| 修复 | `smoke-web-dual-browser.mjs` 改为在 `assertExecutable(GATEWAY_BIN, "gateway")` 之后通过 `await import("playwright")` 延迟加载；缺预构建 gateway 时不再被 Playwright 依赖问题遮蔽 |
| 验证 | `python3 scripts/test_smoke_web_dual_browser_unit.py`、`node --check scripts/smoke-web-dual-browser.mjs` 通过；`SKIP_BUILD=1 GATEWAY_BIN=/tmp/lobster-missing-web-gateway node scripts/smoke-web-dual-browser.mjs` 只输出 gateway 缺失错误，且未创建 `/tmp/lobster-web-dual-browser.*` state 目录 |

### Provider federation 预构建路径收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_smoke_provider_federation_unit.py` 先要求 `smoke-provider-federation.sh` 仅在无 artifact 且需要构建时检查 cargo，并要求 `BIN_PATH` 可执行检查早于 `STATE_ROOT` 创建 |
| 修复 | `smoke-provider-federation.sh` 将 `need_cmd cargo` 移入构建分支；artifact 解包和 gateway binary 可执行检查前置到创建 smoke state 之前；artifact 解包临时目录纳入 cleanup |
| 验证 | `python3 scripts/test_smoke_provider_federation_unit.py`、release gate/coverage quick unit、`bash -n scripts/smoke-provider-federation.sh scripts/smoke-release-gate.sh` 通过；无 cargo PATH 下缺 `BIN_PATH` 或缺 artifact 均给出明确错误，artifact 内 gateway 不可执行时不残留 `/tmp/lobster-chat-smoke.*` |

### Package release 预构建打包路径收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_package_release_unit.py` 先要求 `package-release.sh` 仅在实际构建时检查 cargo；no-cargo `SKIP_BUILD=1` 打包验证同时暴露 source archive 会把 `.git/fsmonitor--daemon.ipc` socket 打进 tar 的警告 |
| 修复 | `package-release.sh` 将 `need_cmd cargo` 移入 `SKIP_BUILD != 1` 构建分支；source archive 新增 `.git` 排除，避免发布包携带 Git 历史、socket 和本地元数据 |
| 验证 | `python3 scripts/test_package_release_unit.py`、`bash -n scripts/package-release.sh` 通过；只提供 `rustc` 不提供 `cargo` 的 PATH 下执行 `SKIP_BUILD=1 DIST_DIR=/tmp/... bash scripts/package-release.sh` 成功生成 source/web/gateway artifacts，且 source tar 内无 `.git` 路径、无 socket 警告 |

### Package release host target override 收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_package_release_unit.py` 先要求 `package-release.sh` 支持 `HOST_TARGET_OVERRIDE`，并且只有未提供 override 时才检查 `rustc`；确认预构建打包路径仍被 rustc 环境硬依赖卡住 |
| 修复 | `package-release.sh` 新增 `HOST_TARGET_OVERRIDE`，用于直接指定 gateway artifact target 名；未设置时继续通过 `rustc -vV` 推断 host target，保持默认构建路径不变 |
| 验证 | `python3 scripts/test_package_release_unit.py`、`bash -n scripts/package-release.sh` 通过；无 `rustc/cargo` PATH 下执行 `SKIP_BUILD=1 HOST_TARGET_OVERRIDE=test-target DIST_DIR=/tmp/... bash scripts/package-release.sh` 成功生成 `lobster-waku-gateway-test-target.tar.gz` |

### Package release archive 体积/本地缓存收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_package_release_unit.py` 先要求 source archive 排除 `.playwright-cli`、根/嵌套 `node_modules`，并要求 H5 shell archive 排除 `node_modules` 与 `backups`；当前项目存在 `apps/lobster-web-shell/node_modules` 约 163M、根 `node_modules` 约 14M、`.playwright-cli` 日志约 4.5M |
| 修复 | `package-release.sh` 的 source tar 新增 `.playwright-cli`、根 `node_modules` 与任意层级 `node_modules` 排除；H5 shell tar 新增 `./node_modules` 与 `./backups` 排除；保留 `generated/`，因为静态 fallback 与测试 fixture 会引用 `generated/bootstrap.json`、`generated/state.json` 与 `generated/state.contract.json` |
| 验证 | `python3 scripts/test_package_release_unit.py`、`bash -n scripts/package-release.sh` 通过；`SKIP_BUILD=1 HOST_TARGET_OVERRIDE=test-target DIST_DIR=/tmp/... bash scripts/package-release.sh` 实测 source tar 不含 `.git`/`.playwright-cli`/`node_modules`/`target`/`dist`，web shell tar 不含 `node_modules`/`backups` 且仍包含 `generated/*.json` |

### Package release runtime artifact 边界收口

| 项目 | 结果 |
|------|------|
| 红灯 | 临时打包检查发现 H5 runtime artifact 仍包含 `screenshots`、`test`、`test-results`、`.tmp` 与根层验收/截图 `.mjs` 脚本；source archive 仍包含前端截图、test-results、backups 与 `.source.html` 生成源文件 |
| 修复 | `package-release.sh` 的 H5 shell tar 新增 `.tmp`、`test`、`test-results`、`screenshots`、根层 `*.mjs`、`.DS_Store` 与 `*.source.html` 排除；source tar 新增任意层级 `backups`、`test-results`、`screenshots`、`.tmp`、`.DS_Store` 与 `*.source.html` 排除，同时保留源码测试目录 |
| 验证 | `python3 scripts/test_package_release_unit.py`、`bash -n scripts/package-release.sh` 通过；临时 `DIST_DIR=/tmp/...` 打包实测 source tar 不含本地生成/备份产物且保留 `apps/lobster-web-shell/test/*.mjs`，web shell tar 不含测试/截图/脚本/缓存并保留运行时页面、JS/CSS、assets 与 generated fallback |

### Restart gateway 预构建入口收口

| 项目 | 结果 |
|------|------|
| 红灯 | `test_restart_gateway_unit.py` 先要求 `restart-gateway.sh` 支持 `SKIP_BUILD` 与 `GATEWAY_BIN`，并在启动/杀进程前检查 binary 可执行；`test_makefile_unit.py` 先要求 `make dev` 不再先 release build 再调用 restart 脚本 |
| 修复 | `restart-gateway.sh` 新增 `SKIP_BUILD`、`GATEWAY_BIN` 与 `need_cmd`，默认仍 build debug gateway；`SKIP_BUILD=1` 时直接使用传入/默认 binary 并提前报缺；`Makefile dev` 收敛为单入口 `./scripts/restart-gateway.sh`，避免 release build 后又 debug build |
| 验证 | `python3 scripts/test_restart_gateway_unit.py`、`python3 scripts/test_makefile_unit.py`、`python3 scripts/test_smoke_release_gate_unit.py`、`bash -n scripts/restart-gateway.sh scripts/smoke-release-gate.sh` 通过；`SKIP_BUILD=1 GATEWAY_BIN=/tmp/lobster-missing-restart-gateway bash scripts/restart-gateway.sh` 提前返回 `gateway binary not found` |

## 2026-06-08 DS v4 Pro Phase 5 完整推进摘要 (3 sessions)

### Bug 修复 (3 项)

| 项目 | 结果 |
|------|------|
| app.js prevMessage 重复声明 | 删除第二个 `const prevMessage`，复用第一个。node --check 通过 |
| Rust 测试 env var 竞争 | 22 个 admin 测试因并行竞争返回 401。Makefile 添加 `--test-threads=1` |
| 骨架头像 `room is not defined` | `room?.id` → `"timeline-skeleton"`（不在函数作用域） |
| 搜索模式全局状态泄漏 (6 测试) | renderRooms/residentList display toggle 添加 `searchModeControlsEl` 守卫 |

### Phase 5-1: 居民检索 + 个人房间入口

| 交付 | 详情 |
|------|------|
| `enterResidentRoom(resident)` | 新 helper：优先使用 `personal_room_id` 直接导航，回退到 `openDirectSession` |
| `renderResidentList()`/`renderResidents()` 收口 | 两处统一使用 `enterResidentRoom` |
| `directRoomPeerOnlineStatus(room)` | 交叉引用 `governance.residents` 获取私聊对象在线状态 |
| 房间头像在线指示器 | CSS `::after` 绿色/灰色圆点 (peer-online/peer-offline) |

### Phase 5-2: 房间 layer 配置 (审查：全链路已完整)
无需额外开发 — SceneImageLayer/HotspotLayer 合同→Gateway→Admin→前端全链路就绪。

### Phase 5-3: 居民搜索 UI 分离

| 交付 | 详情 |
|------|------|
| `searchMode` 变量 + `createSearchModeButton` + `updateSearchModeTabs` | 全部/房间/居民 三模式分段控件 |
| 搜索模式控件 | creative/user 模式搜索框上方 `.creative-search-mode` |
| 显示切换 | rooms-only 隐藏居民列表，residents-only 隐藏房间列表 |
| `bindRoomSearchInput` 增强 | 输入时同步触发 resident list re-render |

### 头像图片渲染 (shell-avatar.js)

| 交付 | 详情 |
|------|------|
| `shell-avatar.js` (NEW, 45行) | djb2 hash → 20 色调色板 → 独特背景色 + 亮度自适应文字色 + 光泽渐变 |
| 5 处渲染接入 | 房间列表、居民列表、peer 消息、self 消息、骨架占位 |
| fake-dom 支持 | 导入映射 + 模块 URL 重写 |

### 测试基线
- JS: **737 pass, 0 fail**
- Rust Gateway: **232 pass, 0 fail** (--test-threads=1)
- **Total: 969 pass, 0 fail**

### 改动文件汇总
| 文件 | 行数 | 说明 |
|------|------|------|
| `shell-avatar.js` | +45 (NEW) | 独立头像样式模块 |
| `app.js` | +150/-14 | Phase 5 全部功能 |
| `styles.css` | +39 | peer-online, creative-search-mode |
| `gateway_tests.rs` | +1 | set_var 恢复 |
| `Makefile` | +1/-1 | --test-threads=1 |
| `test/fake-dom.mjs` | +1 | shell-avatar.js |

### 下一轮建议
1. **TUI parity** — gateway recall/edit/send 端到端测试补齐
2. **Codex 继续技术债** — app.js DOM spec 提取
3. **admin-ds 增强** — 系统日志模块对接审计事件
4. **avatars 进一步** — 支持上传/像素风生成 (需后端端点)

## 2026-06-06 Codex 技术债推进摘要

### Web Shell room inline preview container renderer 复用收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-pages-static.test.mjs` 新增 `room inline preview containers share one DOM renderer`，切 `createRoomInlineActions()` 内部 meta/controls/fieldRows/actions renderer，确认缺少 `createInlineCardContainerNode()`、meta 未复用 helper、row 未复用 helper 时失败 |
| container renderer 收口 | `createRoomInlineActions()` 新增局部 `createInlineCardContainerNode(containerSpec)`，统一消费 `className`、可选 `hidden` 与可选 `ariaHidden`；meta container、controls group、field rows list、field row、actions container 不再各自手写基础容器创建 |
| 防回归测试 | 新增 1 条静态断言，要求 meta 使用 `createInlineCardContainerNode(inlineMetaDomModel)`，controls 使用 `createInlineCardContainerNode(group)`，fieldRows list 使用 `createInlineCardContainerNode(inlineFieldRowsDomModel)`，field row 使用 `createInlineCardContainerNode(rowSpec)`，actions 使用 `createInlineCardContainerNode(inlineActionDomModel)` |
| 验证 | 红灯：缺少 `createInlineCardContainerNode()` 时报错，随后 meta 未复用 helper、row 未复用 helper 均时报错；绿灯：`shell-pages-static.test.mjs` 53 passed；相关 labels/preview/static/fake-dom 测试 304 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 736 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8654 → 8660 → 8659 → 8658；本轮净增 4 行，用一个支持 hidden/ariaHidden 的容器入口换掉 meta/controls/fieldRows/actions 五处基础容器创建重复 |

### Web Shell room inline preview button renderer 复用收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-pages-static.test.mjs` 新增 `room inline preview buttons share one DOM renderer`，切 `createRoomInlineActions()` 内部 controls/actions renderer，确认缺少 `createInlineCardButtonNode()` 时失败 |
| button renderer 收口 | `createRoomInlineActions()` 新增局部 `createInlineCardButtonNode(buttonSpec)`，统一消费 `type/buttonType/dataset/text/title/ariaLabel/clickable`；controls 和 actions 只保留各自点击语义，不再重复初始化 button DOM 属性 |
| 防回归测试 | 新增 1 条静态断言，要求 controls/actions 都通过 `createInlineCardButtonNode(buttonSpec)` 创建按钮；同步调整 clickable 测试，让它约束共享 helper 消费 `applyInlineClickableDomSpec(button, buttonSpec.clickable)` |
| 验证 | 红灯：缺少 `createInlineCardButtonNode()` 时报错；绿灯：`shell-pages-static.test.mjs` 52 passed；相关 labels/preview/static/fake-dom 测试 303 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 735 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8657 → 8654；本轮净减 3 行，把 inline preview controls/actions button 属性初始化收敛为单一路径 |

### Web Shell room inline preview simple child renderer 复用收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-pages-static.test.mjs` 新增 `room inline preview simple children share one DOM renderer`，切 `createRoomInlineActions()` 内部 header / fieldRows renderer，确认缺少 `createInlineCardSimpleChildNode()` 时失败 |
| simple child renderer 收口 | `createRoomInlineActions()` 新增局部 `createInlineCardSimpleChildNode(childSpec)`，统一消费 `childSpec.type || "div"`、`className` 与 `text || ""`；inline preview header 与 field row 子节点不再各自手写同一套 DOM 创建逻辑 |
| 防回归测试 | 新增 1 条静态断言，要求 header 使用 `inlineCard.appendChild(createInlineCardSimpleChildNode(childSpec))`，field rows 使用 `row.appendChild(createInlineCardSimpleChildNode(childSpec))`；同步调整上一条 header generic child 测试，让它约束共享 helper 路径而非局部内联实现 |
| 验证 | 红灯：缺少 `createInlineCardSimpleChildNode()` 时报错；绿灯：`shell-pages-static.test.mjs` 51 passed；相关 labels/preview/static/fake-dom 测试 302 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 734 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8657 → 8657；本轮行数持平，消掉 header / fieldRows 两处简单 child DOM 创建重复，为后续继续统一 inline card 子渲染器留下单一入口 |

### Web Shell room inline preview header generic child render spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-pages-static.test.mjs` 新增 `room inline preview header consumes generic child render specs`，切 `createRoomInlineActions()` 内部 header renderer，确认仍用 `createLine(childSpec.className, childSpec.text)`、未消费 `childSpec.type` 时失败 |
| header 消费收口 | `createRoomInlineActions()` 的 inline preview header renderer 现在按 `childSpec.type || "div"` 创建子节点，并使用 `childSpec.text || ""` 写入文案；render model 输出的通用 child DOM spec 不再只停留在 helper 层 |
| 防回归测试 | 新增 1 条静态断言，锁住 header renderer 对 `type/text` 的通用消费路径，防止后续重新退回 `createLine(className, text)` 的局部协议 |
| 验证 | 红灯：header renderer 未消费 `childSpec.type` 时报错；绿灯：`shell-pages-static.test.mjs` 50 passed；相关 labels/preview/static/fake-dom 测试 301 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 733 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8654 → 8657；本轮微增 3 行，把 header 子节点创建从局部 `createLine()` 迁到通用 child render spec 消费，为后续提取统一 child renderer 铺路 |

### Web Shell room inline preview controls/actions clickable render spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-pages-static.test.mjs` 新增 `room inline preview controls and actions consume clickable render specs`，切 `createRoomInlineActions()` 内部 controls/actions renderer，确认缺少 `applyInlineClickableDomSpec(button, buttonSpec.clickable)` 时失败 |
| clickable 消费收口 | `createRoomInlineActions()` 的 inline preview controls 与 CTA actions 现在都消费 render model 的 `buttonSpec.clickable`，统一套用 `is-clickable`、`tabIndex`、`role/title/aria-label` 等可访问性规格 |
| 防回归测试 | 新增 1 条静态断言，防止后续 render model 已生成 clickable 但主入口漏消费；既有 preview helper 测试继续覆盖 clickable spec 内容 |
| 验证 | 红灯：controls renderer 缺少 clickable 消费时报错；绿灯：`shell-pages-static.test.mjs` 49 passed；相关 labels/preview/static/fake-dom 测试 300 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 732 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8652 → 8654；本轮微增 2 行，换来 controls/actions button 的可访问性规格与 hint/meta 路径一致 |

### Web Shell room inline preview field row children render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewFieldRowsRenderDomModel` 测试，要求字段行 child 输出通用 `type/text` render spec，确认缺失 `type/text` 时失败 |
| field row children 抽取 | `buildQuickActionInlinePreviewFieldRowsRenderDomModel()` 现在在保留 `labelNode/valueNode` 兼容字段的同时，将 row children 规范化为 `{ type, className, label, text }` |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview field rows 分支改为消费 `childSpec.type/text` 创建子节点，不再依赖字段行专属 `label` 读取路径 |
| 防回归测试 | 增强 1 条 field rows render DOM spec 测试，覆盖 label/value 子节点的 `type/text` 和空 value 的 `待补充` fallback |
| 验证 | 红灯：字段行 child 缺少 `type/text` 报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-quick-action-preview.test.mjs` 通过；`npm test` 731 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8649 → 8652；本轮微增 3 行，但字段行子节点渲染已改为通用 render spec 消费，为后续统一 inline card child renderer 铺路 |

### Web Shell room inline preview card children render order 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewCardRenderDomModel` 测试，要求 card render model 输出有序 `children`，确认 `children` 缺失时报错 |
| card children 抽取 | `buildQuickActionInlinePreviewCardRenderDomModel()` 现在在保留 `header/meta/controls/fieldRows/actions` 兼容字段的同时，生成 `children`，明确 `header:before-meta → meta → header:after-meta → controls → fieldRows → actions` 顺序 |
| `app.js` 收口 | `createRoomInlineActions()` 改为遍历 `inlineCardChildren` 并按 kind 分发渲染；事件绑定仍保留在主文件，但 inline card 子模块插入顺序由 render model 控制 |
| CSS split 测试债 | `shell-pages-static.test.mjs` 新增 `readAdminShellCss()`，让 admin 静态断言读取 `styles.css + styles.admin.css`，修复 CSS 拆分后测试仍只看主 CSS 导致的 admin selector 假失败 |
| 防回归测试 | 增强 1 条 card render DOM spec 测试，覆盖有序 children 与 header placement 文案；`shell-pages-static.test.mjs` 继续覆盖 admin nav collapse、workspace panel、action-status 高对比样式 |
| 验证 | 红灯：`model.children` 缺失时报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；`shell-pages-static.test.mjs` 48 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js`、`shell-quick-action-preview.js`、`shell-pages-static.test.mjs` 通过；`npm test` 731 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8639 → 8649；本轮因主文件新增 kind 分发函数微增 10 行，但 inline card 子模块顺序规则已从主入口下沉到 render model |

### Web Shell room inline preview header children render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewHeaderDomModel` 测试，要求 header 生成带 placement 的 `children` line 节点规格，确认 `children/placement` 缺失时报错 |
| header children 抽取 | `buildQuickActionInlinePreviewHeaderDomModel()` 现在在保留原 `lines` 兼容字段的同时，生成 `children`，明确 stage/summary line 的 type、key、placement、className 与 text |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview header 分支改为按 `before-meta` / `after-meta` placement 消费 children，保持原 stage → meta → summary DOM 顺序，不再直接按 `line.key` 分两段过滤 |
| 防回归测试 | 增强 1 条 header DOM spec 测试，覆盖 stage/summary children 和 placement，避免后续重构改变 inline card 顺序 |
| 验证 | 红灯：`children` 缺失、随后 `placement` 缺失均时报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-preview.js` 通过；`npm test` 731 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8643 → 8639；本轮减少 4 行，并把 inline preview header 子节点与插入位置规则下沉到 render model |

### Web Shell room inline preview meta children render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewMetaRenderDomModel` 测试，要求 meta section 生成 label/container/pill 的 `children` 节点规格，确认 `children` 缺失时报错 |
| meta children 抽取 | `buildQuickActionInlinePreviewMetaRenderDomModel()` 现在在保留原 `labelNode/container/pills` 兼容字段的同时，生成 `children`，明确 label、container、currentStrip、pill 的 type、className、dataset、text、actionTarget 与 clickable |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview meta 分支改为消费 `section.children` 递归创建节点，不再本地手工拼 label/container/currentStrip/pill DOM 结构 |
| 防回归测试 | 增强 1 条 meta render DOM spec 测试，覆盖 history section 的完整子节点结构、pill actionTarget 与 clickable 可访问性规格 |
| 验证 | 红灯：`children` 缺失时报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-preview.js` 通过；`npm test` 731 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8661 → 8643；本轮减少 18 行，并把 inline preview meta 分区子节点结构规则下沉到 render model |

### Web Shell room inline preview action children render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewActionRenderDomModel` 测试，要求 CTA actions 生成明确的 `children` button 节点规格，确认 `children` 缺失时报错 |
| action children 抽取 | `buildQuickActionInlinePreviewActionRenderDomModel()` 现在在保留原 `buttons` 兼容字段的同时，生成 `children`，明确 button 的 type、buttonType、dataset、text、title、ariaLabel、actionTarget 与 clickable |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview CTA 分支改为消费 `inlineActionDomModel.children` 创建按钮，不再把 `buttons` 同时当业务按钮模型和 DOM 节点规格解释 |
| 防回归测试 | 增强 1 条 action render DOM spec 测试，覆盖 snapshot CTA child 的完整节点结构、actionTarget 与 clickable 可访问性规格 |
| 验证 | 红灯：`children` 缺失时报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-preview.js` 通过；`npm test` 731 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8661 → 8661；本轮主入口行数持平，但 CTA button DOM 节点解释规则已从主入口下沉到 render model |

### Web Shell room inline preview controls children render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先增强 `shell-quick-action-preview.test.mjs` 的 `buildQuickActionInlinePreviewControlsRenderDomModel` 测试，要求 controls group 生成明确的 `children` button 节点规格，确认字段缺失时报错 |
| controls children 抽取 | `buildQuickActionInlinePreviewControlsRenderDomModel()` 现在在保留原 `buttons` 兼容字段的同时，生成 `children`，明确 button 的 type、buttonType、dataset、text、title、actionTarget 与 clickable |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview controls 分支改为消费 `group.children` 创建按钮，不再把 `group.buttons` 同时当业务按钮模型和 DOM 节点规格解释 |
| 防回归测试 | 增强 1 条 controls render DOM spec 测试，覆盖 history button child 的完整节点结构、actionTarget 与 clickable 可访问性规格 |
| 验证 | 红灯：`children` 缺失断言失败；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-preview.js` 通过；`npm test` 721 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8661 → 8661；本轮主入口行数持平，但 controls button DOM 节点解释规则已从主入口下沉到 render model |

### Web Shell room inline progress render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` 新增 `buildRoomInlineProgressRenderDomSpec` 导入与组合规格测试，确认缺少导出时报错 |
| progress render 抽取 | 新增 `buildRoomInlineProgressRenderDomSpec()`，在原 progress DOM spec 基础上组合容器、count 子节点和 label 子节点 |
| `app.js` 收口 | `createRoomInlineActions()` 不再分别手工创建 progress count / label 两个 span，改为消费 `children` render spec |
| 防回归测试 | 新增 2 条 progress render DOM spec 测试，覆盖委托中间阶段的完整子节点组合与空输入返回 null |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-labels.test.mjs` 67 passed；相关 labels/preview/static/fake-dom 测试 299 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-labels.js` 通过；`npm test` 721 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8665 → 8661；本轮减少 4 行，并把 inline progress 子节点组合规则下沉到纯 helper |

### Web Shell room inline primary/secondary action DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` 新增 `buildRoomInlineActionDomSpec` 导入与 primary/secondary 节点规格测试，确认缺少导出时报错 |
| action 节点抽取 | 新增 `buildRoomInlineActionDomSpec()`，统一房间内联 primary/secondary 动作节点的 tag、class、dataset、tabIndex、role 与文案规格 |
| `app.js` 收口 | `createRoomInlineActions()` 的底部 `appendAction()` 不再本地写死 action span 结构，改为消费纯 DOM spec；点击事件与业务行为仍保留在主文件 |
| 防回归测试 | 新增 2 条 action DOM spec 测试，覆盖 primary、secondary、空 label/role 返回 null，以及未知 action 不写 `actionIntensity` |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-labels.test.mjs` 65 passed；相关 labels/preview/static/fake-dom 测试 297 passed；`node --check apps/lobster-web-shell/app.js` 与 `shell-quick-action-labels.js` 通过；`npm test` 719 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8658 → 8665；本轮因通用 spec 消费和属性映射增加 7 行，但 primary/secondary action 节点结构规则已从主入口下沉到纯 helper |

### Web Shell room inline preview panel render DOM model 组合收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewPanelRenderDomModel` 导入与组合规格测试，确认缺少导出时报错 |
| 组合 render 抽取 | 新增 `buildQuickActionInlinePreviewPanelRenderDomModel()`，统一组合 inline preview 的 hint render DOM model 与 card render DOM model |
| `app.js` 收口 | `createRoomInlineActions()` 不再分别调用 hint render 与 card render helper，只消费一个 panel render model |
| 防回归测试 | 新增 2 条 panel render DOM spec 测试，覆盖 hint dataset、card/header/meta/controls/fieldRows/actions 组合与空输入 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 181 passed；相关 labels/preview/static/fake-dom 测试 295 passed；`node --check apps/lobster-web-shell/shell-quick-action-preview.js` 与 `app.js` 通过；`npm test` 717 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8663 → 8658；本轮减少 5 行，并把 inline preview panel 的 render 组合关系下沉到纯 helper |

### Web Shell room inline preview card render DOM model 组合收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewCardRenderDomModel` 导入与组合规格测试，确认缺少导出时报错 |
| 组合 render 抽取 | 新增 `buildQuickActionInlinePreviewCardRenderDomModel()`，统一组合 inline card 的 card/header/meta/controls/fieldRows/actions render DOM model |
| `app.js` 收口 | `createRoomInlineActions()` 不再分别调用 header/meta/controls/action/fieldRows 五个 DOM helper，只消费一个组合 card render model |
| 防回归测试 | 新增 2 条组合 render DOM spec 测试，覆盖 card、header stage/summary、meta 分区、controls target、fieldRows children、actions target 与空输入 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 179 passed；相关 labels/preview/static/fake-dom 测试 293 passed；`node --check apps/lobster-web-shell/shell-quick-action-preview.js` 与 `app.js` 通过；`npm test` 715 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8676 → 8663；本轮减少 13 行，并把 inline card DOM model 组合关系下沉到纯 helper |

### Web Shell room inline preview controls render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewControlsRenderDomModel` 导入与规格测试，确认缺少导出时报错 |
| controls render 抽取 | 新增 `buildQuickActionInlinePreviewControlsRenderDomModel()`，在 controls DOM model 基础上预解析 history / field-view button 的 `actionTarget` 与 clickable 可访问性规格 |
| 空输入修复 | `buildQuickActionInlinePreviewControlsDomModel(null)` 现在安全返回 `null`，避免无效输入直接抛错 |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview controls 分支不再本地调用 `quickActionInlinePreviewControlActionTarget()`，改为消费 `buttonSpec.actionTarget` |
| 防回归测试 | 新增 2 条 controls render DOM spec 测试，覆盖 history target、field-view target、aria/title 规格、无效 target 与空输入 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 177 passed；相关 labels/preview/static/fake-dom 测试 291 passed；`node --check apps/lobster-web-shell/shell-quick-action-preview.js` 与 `app.js` 通过；`npm test` 713 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8676 → 8676；本轮主入口行数持平，controls target 解释规则已从主文件下沉到纯 helper |

### Web Shell room inline preview action / field rows render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewActionRenderDomModel` / `buildQuickActionInlinePreviewFieldRowsRenderDomModel` 导入与规格测试，确认缺少导出时报错 |
| action render 抽取 | 新增 `buildQuickActionInlinePreviewActionRenderDomModel()`，在 CTA DOM model 基础上预解析 snapshot/workflow 的 `actionTarget` 与 clickable 可访问性规格 |
| field rows render 抽取 | 新增 `buildQuickActionInlinePreviewFieldRowsRenderDomModel()`，把字段行 label/value 子节点组合为稳定 `children` render 规格 |
| `app.js` 收口 | `createRoomInlineActions()` 的 inline preview 底部 CTA 不再本地调用 `quickActionInlinePreviewActionTarget()`；字段行不再直接读取 `labelNode/valueNode`，改为消费 render model |
| 防回归测试 | 新增 4 条 render DOM spec 测试，覆盖 CTA target、clickable aria/title 规格、无效 action 过滤、字段行子节点组合与空输入 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 175 passed；相关 labels/preview/static/fake-dom 测试 289 passed；`node --check apps/lobster-web-shell/shell-quick-action-preview.js` 与 `app.js` 通过；`npm test` 711 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8675 → 8676；本轮因 helper 名称更长微增 1 行，但 CTA target 与字段行 render 组合规则已从主文件下沉到纯 helper |

## 2026-06-05 Codex 技术债推进摘要

### Web Shell room inline preview meta render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewMetaRenderDomModel` 导入与组合规格测试，确认缺少导出时报错 |
| meta render 抽取 | 新增 `buildQuickActionInlinePreviewMetaRenderDomModel()`，在 meta DOM model 基础上预解析 history / field-view pill 的 `actionTarget` 与 clickable 可访问性规格 |
| `app.js` 收口 | `createRoomInlineActions()` 的 meta 分支不再本地调用 `quickActionInlinePreviewMetaActionTarget()` 或自行生成 clickable spec，只按 `pillSpec.actionTarget/clickable` 绑定 click/keydown 行为 |
| 防回归测试 | 新增 2 条 meta render DOM spec 测试，覆盖 history target、field-view target、aria/title 规格与无效 action 过滤 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 171 passed；相关 labels/preview/room-rail/static/fake-dom 测试 425 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 707 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8675 → 8675；本轮主文件行数持平，但 meta action target 与 clickable 规则已从主文件下沉到纯 helper |

### Web Shell room inline preview hint render DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionInlinePreviewHintRenderDomModel` 导入与组合规格测试，确认缺少导出时报错 |
| hint render 抽取 | 新增 `buildQuickActionInlinePreviewHintRenderDomModel()`，在 hint DOM model 基础上预解析 `actionTarget` 与 clickable 可访问性规格 |
| `app.js` 收口 | `createRoomInlineActions()` 的 hint 分支不再本地调用 `quickActionInlinePreviewHintActionTarget()` 解释 action，只消费 render model 上的 `part.actionTarget/part.clickable`；事件绑定仍留在主文件 |
| 防回归测试 | 新增 2 条 hint render DOM spec 测试，覆盖 workflow/snapshot/history target、clickable aria/title 规格与无效 action 过滤 |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-preview.test.mjs` 169 passed；相关 labels/preview/room-rail/static/fake-dom 测试 423 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 705 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8677 → 8675；本轮减少 2 行，同时把 hint action target 解释从主文件下沉到纯 helper |

### Web Shell room inline actions rail DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` 新增 `buildRoomInlineActionsRailDomSpec` 导入与规格测试，确认缺少导出时报错 |
| rail 容器抽取 | 新增 `buildRoomInlineActionsRailDomSpec()`，统一 `room-inline-actions` 容器 class 与 `quickAction/actionIntensity` dataset 规格 |
| `app.js` 收口 | `createRoomInlineActions()` 不再本地写死 rail 容器 class 与 action dataset，改为消费纯 DOM spec；后续 hint/meta/action/fieldRows 渲染仍待继续拆 |
| 防回归测试 | 新增 2 条 rail DOM spec 测试，覆盖已知 action、未知自定义 action、空 action 返回 null |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-labels.test.mjs` 63 passed；相关 labels/preview/room-rail/static/fake-dom 测试 421 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 703 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8674 → 8677；本轮因通用 spec 消费微增 3 行，但 rail 容器 UI 规则已从主文件下沉到纯 helper |

### Web Shell room inline progress DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` 新增 `buildRoomInlineProgressDomSpec` 导入与规格测试，确认缺少导出时报错 |
| inline progress 抽取 | 新增 `buildRoomInlineProgressDomSpec()`，统一房间 inline action 进度条的 class、dataset、title、tabIndex、role、计数和状态标签规格 |
| `app.js` 收口 | `createRoomInlineActions()` 不再本地计算 progress 的 stageIndex、count 文案、label class 与 actionIntensity dataset；点击预览行为仍保留在 `app.js` |
| 防回归测试 | 新增 2 条 inline progress DOM spec 测试，覆盖委托中间阶段、未知状态回退第一阶段、未知 action 返回 null |
| 验证 | 红灯：缺少导出时报错；绿灯：`shell-quick-action-labels.test.mjs` 61 passed；相关 labels/preview/room-rail/static/fake-dom 测试 419 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 701 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8673 → 8674；本轮因通用 spec 消费与 dataset 写入微增 1 行，但进度条 UI 规则已从主文件下沉到纯 helper |

### Web Shell room quick action pill DOM spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` / `shell-quick-action-preview.test.mjs` 新增 `buildRoomQuickActionPillDomSpec` 与 `buildRoomQuickPreviewPillDomSpec` 导入和规格测试，确认缺少导出时报错 |
| quick action pill 抽取 | 新增 `buildRoomQuickActionPillDomSpec()`，统一房间动作 pill 的 text/tone/class/dataset/title 规格 |
| preview pill 抽取 | 新增 `buildRoomQuickPreviewPillDomSpec()`，统一房间预览 pill 的轮次文案、字段视图文案、tone、dataset 与 title 规格 |
| `app.js` 收口 | `createRoomQuickActionPill()` / `createRoomQuickPreviewPill()` 改为消费纯 DOM spec；点击事件、房间聚焦与 composer seed 行为保留在 `app.js` |
| 防回归测试 | 新增 4 条 pill DOM spec 测试，覆盖动作 pill、空 action、最新/历史预览轮次、缺失 historyLabel |
| 验证 | 红灯：两个 helper 缺少导出时报错；绿灯：相关 labels/preview/room-rail/static/fake-dom 测试 417 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 699 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8675 → 8673 |

### Web Shell quick action preview card render spec 收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionPreviewCardRenderDomSpec` 导入与组合规格测试，确认缺少导出时报错 |
| preview card 抽取 | 新增 `buildQuickActionPreviewCardRenderDomSpec()`，统一组合普通 preview card 的 card/header/pills/copy/controlPanels/sheet render spec |
| `app.js` 收口 | `createQuickActionPreviewCard()` 不再直接串联 card/header/pills/copy/history/field-view/sheet 低层 helper，只消费组合 render spec；事件绑定与真实 DOM 创建仍留在 `app.js` |
| 防回归测试 | 新增 2 条 render spec 测试，覆盖 dataset 折叠 flag、pill wrapper、三类 pill 分区、copy、history/field-view 控制面板与 notes sheet |
| 验证 | 红灯：缺少导出时报错；绿灯：`node --test apps/lobster-web-shell/test/shell-quick-action-preview.test.mjs` 165 passed；相关静态/fake-dom 测试 216 passed；`node --check apps/lobster-web-shell/app.js` 通过；`npm test` 695 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8731 → 8675 |

### Web Shell quick action preview 控制区收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-preview.test.mjs` 新增 `buildQuickActionPreviewControlPanelsRenderDomSpec` 测试，确认缺少导出时失败 |
| preview 控制区抽取 | 新增 `buildQuickActionPreviewControlPanelsRenderDomSpec()`，统一组合普通 preview card 的 history / field-view 控制面板 render spec |
| `app.js` 收口 | `createQuickActionPreviewCard()` 删除 history / field-view 两段重复 wrapper/button DOM 翻译逻辑，改为消费统一 panel spec；事件绑定仍留在 `app.js` |
| 防回归测试 | `shell-quick-action-preview.test.mjs` 新增复合控制面板测试，覆盖 wrapper、labelLine、buttonsClassName 与 actionTarget |
| 验证 | 红灯：缺少导出时报错；绿灯：`node --test apps/lobster-web-shell/test/shell-quick-action-preview.test.mjs` 163 passed；`node --check apps/lobster-web-shell/app.js` 通过；相关静态/fake-dom 测试 214 passed；`npm test` 最终 693 unit passed / 0 failed，layout passed，realness passed |
| 备注 | 第一次全量 `npm test` 出现一次 `hub shell keeps local-memory gateway composer online without upstream provider` 并跑抖动；该测试单跑 8/8 通过，随后全量复跑通过 |
| `app.js` 行数 | 8749 → 8731 |

### Web Shell quick action / admin-ds 安全债收口

| 项目 | 结果 |
|------|------|
| TDD 红灯 | 先在 `shell-quick-action-labels.test.mjs` 新增 `buildWorkflowProgressDomSpec` 规格测试，确认缺少导出时失败 |
| quick action 抽取 | 新增 `buildWorkflowProgressDomSpec()`，把 workflow progress 的 class/dataset/title/step 规格从 `app.js` 收到纯 helper；`createWorkflowProgress()` 只保留 DOM 创建与事件绑定 |
| admin-ds 安全修复 | 修掉 `loadDevices()` 的 `tbody.innerHTML = ...` 加载/空/失败状态，改为 `clear()` + `el()` + `textContent` |
| 防回归测试 | `shell-quick-action-labels.test.mjs` 追加 3 条 workflow progress DOM spec 边界测试；既有 `admin-ds` 静态测试继续禁止 `tbody.innerHTML` |
| 验证 | `node --test apps/lobster-web-shell/test/shell-quick-action-labels.test.mjs`：57 passed / 0 failed；`node --test apps/lobster-web-shell/test/admin-ds-static.test.mjs apps/lobster-web-shell/test/admin-ds-runtime.test.mjs`：35 passed / 0 failed；`npm test`：692 unit passed / 0 failed，layout passed，realness passed |
| `app.js` 行数 | 8758 → 8749 |

### 本轮改动文件

| 文件 | 说明 |
|------|------|
| `apps/lobster-web-shell/shell-quick-action-labels.js` | 新增 workflow progress DOM spec 纯 helper |
| `apps/lobster-web-shell/test/shell-quick-action-labels.test.mjs` | 新增 helper 单测，覆盖默认 action、自定义 stages、无 stages |
| `apps/lobster-web-shell/app.js` | `createWorkflowProgress()` 改为消费纯 spec，降低本地规则计算 |
| `apps/lobster-web-shell/admin-ds.js` | 设备管理表格状态改用安全 DOM API，恢复静态安全测试 |

### 下一轮建议

1. 继续从 `createQuickActionPreviewCard()` 拆普通 preview card 的控制区 DOM renderer，优先抽无状态 DOM spec 到 `shell-quick-action-preview.js`。
2. 若 DS/CC 正在改 H5 交互，Codex 可转向 Rust gateway/TUI 合同测试，避免并发碰 `app.js`。

## 2026-06-04 Codex 技术债推进摘要

### Gateway 持久化基线修复

| 项目 | 结果 |
|------|------|
| 根因 | `SceneImageLayer.day_image_url/night_image_url` 在 durable postcard schema 上使用 `skip_serializing_if`，当默认 scene 的两个字段为 `None` 时会省略 Option discriminant，导致重启读取 `conversations.postcard` 时布局错位并被 quarantine |
| 修复 | `chat-core` 保留 `#[serde(default)]`，移除两个 Option 字段的 `skip_serializing_if`，确保存储快照始终写出稳定二进制布局 |
| 防回归测试 | `chat-storage` 的 scene metadata roundtrip 覆盖 `image_layer: Some(...)` 且 day/night 均为 `None`；Gateway 新增 `seeded_conversations_persist_across_restart`，验证种子会话重启后不产生 `conversations.postcard.corrupt-*` |
| 验证 | `cargo test -p chat-storage`：13 passed / 0 failed；`cargo test -p lobster-waku-gateway`：232 passed / 0 failed；`cargo test -p lobster-tui`：212 passed / 0 failed；`cargo test -p lobster-cli`：28 passed / 0 failed |
| 已恢复的红灯 | `runtime_persists_shell_messages_across_restart`、`edit_and_recall_state_persists_across_restart`、`email_otp_verification_seeds_canonical_guide_direct_conversation` |

## 2026-06-02 DS v4 Pro 执行摘要

### 本轮完成

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| P0 | H5 IM 主路径真实验收 (双端消息闭环) | 完成 |
| P1 | Gateway 合同加固 + 审核持久化 + 边界测试 | 完成 |
| P3 | admin-ds 审核 (create-permission-group 需后端设计，已标注) | 完成 |
| P4 | 多页面左栏一致性验证 (已统一 220px) | 完成 |
| P5 | TUI/CLI parity 测试通过 | 完成 |

### 核心交付

| 交付 | 详情 |
|------|------|
| 审核持久化 | `message_moderation` HashMap → `moderation-state.json` 文件持久化，atomic write |
| 新增 Gateway 测试 | 审核持久化、readback、不存在房间拒绝、并发 presence HTTP |
| H5 双端验收 | qa-a ↔ qa-b 公共房间 + 直聊收发验证通过，未读标记正常 |
| 测试基线 | Gateway 214, TUI 195, CLI 16, Web Shell 688 (总计 1113, 全部 0 fail) |

### 改动文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `gateway_models.rs` | 修改 | 新增 `moderation_state_path: PathBuf` |
| `core_runtime.rs` | 修改 | 新增 `persist_moderation_state()` / `load_moderation_state()`，审核后自动持久化 |
| `gateway_tests.rs` | 新增 4 测试 | moderation_state_persists, send_to_nonexistent_room, admin_moderation_status_readback, concurrent_presence_http |
| `ACTIVE_WORK_QUEUE.md` | 更新 | 本轮摘要

### 技术债压降追加

| 项目 | 结果 |
|------|------|
| `app.js` 房间搜索重复实现 | 已删除本地 `roomMatchesSearch()`，统一委托 `shell-room-rail.js` |
| 防回归测试 | `shell-pages-static.test.mjs` 新增静态约束，禁止 `app.js` 重新保留搜索实现 |
| 角色权限 helper | 新增 `shell-role-permissions.js`，抽出 `roleAllows*` 纯权限判断 |
| 权限 helper 防回归测试 | 新增 `shell-role-permissions.test.mjs`，覆盖 Lord/Steward/Resident/空角色权限边界 |
| quick action follow-up helper | `quickActionFollowUpLabel/Copy` 从 `app.js` 移入 `shell-quick-action-labels.js` |
| quick action 标签测试 | `shell-quick-action-labels.test.mjs` 新增 3 条 follow-up helper 边界测试 |
| quick action badge helper | `quickActionBadgeLabel/Tone/Intensity` 从 `app.js` 移入 `shell-quick-action-labels.js` |
| quick action badge 测试 | `shell-quick-action-labels.test.mjs` 新增 3 条 badge helper 边界测试 |
| quick action summary/context helper | `quickActionSummary/ContextCopy` 从 `app.js` 文案拼接移入 `shell-quick-action-labels.js` |
| quick action summary/context 测试 | `shell-quick-action-labels.test.mjs` 新增 4 条 summary/context helper 边界测试 |
| quick action 状态推进 helper | `nextQuickActionState` 从 `app.js` 移入 `shell-quick-action-labels.js` |
| quick action 状态推进测试 | `shell-quick-action-labels.test.mjs` 新增 3 条 next-state 边界测试 |
| quick action 默认发送文案 helper | `quickActionDefaultSendLabel` 从 `app.js` switch 移入 `shell-quick-action-labels.js` |
| quick action 默认发送文案测试 | `shell-quick-action-labels.test.mjs` 新增 2 条 send-label 边界测试 |
| workflow progress 阶段状态 helper | `workflowProgressStageState` 从 `app.js` 移入 `shell-quick-action-labels.js` |
| workflow progress 阶段状态测试 | `shell-quick-action-labels.test.mjs` 新增 3 条 stage-state 边界测试 |
| quick action 结构化草稿 helper | `quickActionStructuredDraft` 从 `app.js` 移入 `shell-quick-actions.js` |
| quick action runtime 测试 | 新增 `shell-quick-actions.test.mjs`，覆盖结构字段草稿、默认模板、合同模板覆盖 |
| quick action preview 视图规则 helper | `quickActionPreviewDefaultFieldView` / `quickActionPreviewHistoryToneClass` 从 `app.js` 移入 `shell-quick-action-preview.js` |
| quick action preview 视图规则测试 | `shell-quick-action-preview.test.mjs` 新增 6 条默认视图 / 历史轮次 tone 边界测试 |
| quick action preview 记录视图 helper | `quickActionPreviewResolvedSnapshotIndex` / `quickActionPreviewSelectedFieldView` 抽出 record 与 snapshot 选择规则 |
| quick action preview 记录视图测试 | `shell-quick-action-preview.test.mjs` 新增 6 条 snapshot index / record fieldView 边界测试 |
| quick action preview state/index helper | `quickActionPreviewSelectedState` / `quickActionPreviewSelectedSnapshotIndex` 抽出 preview 状态与快照索引选择规则 |
| quick action preview state/index 测试 | `shell-quick-action-preview.test.mjs` 新增 5 条 state / snapshot index 边界测试 |
| quick action snapshot history helper | `quickActionSnapshotHistoryFromRecord` / `quickActionSnapshotFromHistory` 抽出 snapshot history 记录读取与快照选择规则 |
| quick action snapshot history 测试 | `shell-quick-action-preview.test.mjs` 新增 4 条 snapshot history / snapshot selection 边界测试 |
| quick action preview view helper | `resolveQuickActionPreviewView` 抽出 snapshot/stage preview 展示模型组装规则 |
| quick action preview view 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条 snapshot/stage/null preview view 边界测试 |
| quick action preview model helper | `buildQuickActionPreviewModel` 抽出 room preview model 组装规则 |
| quick action preview model 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条 preview model / history tone / null 边界测试 |
| quick action preview card model helper | `buildQuickActionPreviewCardModel` 抽出 preview card 历史索引、字段视图与 active structured 选择规则 |
| quick action preview card model 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条 card model / fallback / null 边界测试 |
| quick action preview card chrome helper | `buildQuickActionPreviewCardChromeModel` 抽出 preview card 顶部当前条、历史轮次、字段视图 toggle 与折叠状态规则 |
| quick action preview card chrome 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条 current strip / history toggle / safe fallback 边界测试 |
| quick action inline preview card model helper | `buildQuickActionInlinePreviewCardModel` 抽出 inline preview card 字段集、字段视图与摘要选择规则 |
| quick action inline preview card model 测试 | `shell-quick-action-preview.test.mjs` 新增 4 条 latest/history/resolved/null 边界测试 |
| quick action inline preview meta model helper | `buildQuickActionInlinePreviewMetaModel` 抽出 inline preview meta pill 当前条、轮次选项、字段视图选项、切换标题与折叠状态规则 |
| quick action inline preview meta model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条多轮/单轮、history/field-view toggle 边界测试 |
| quick action inline preview controls model helper | `buildQuickActionInlinePreviewControlsModel` 抽出 inline preview 历史按钮、字段视图按钮、hidden/aria-hidden 与 dataset 规则 |
| quick action inline preview controls model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条多轮按钮组/单轮空按钮组边界测试 |
| quick action inline preview action model helper | `buildQuickActionInlinePreviewActionModel` 抽出 inline preview 底部 snapshot/workflow CTA 顺序、默认态、优先级与提示文案规则 |
| quick action inline preview action model 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条可推进阶段/不可推进阶段/历史轮 CTA 顺序边界测试 |
| quick action inline preview hint model helper | `buildQuickActionInlinePreviewHintModel` 抽出 inline preview 顶部阶段、主字段、轮次 hint 与下一轮切换规则 |
| quick action inline preview hint model 测试 | `shell-quick-action-preview.test.mjs` 新增 3 条多轮/单轮/缺失输入边界测试 |
| quick action inline preview field rows model helper | `buildQuickActionInlinePreviewFieldRowsModel` 抽出 inline preview 字段行 label/value 规范化与空值“待补充”回退规则 |
| quick action inline preview field rows model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条字段行规范化/空输入边界测试 |
| quick action inline preview meta sections model helper | `buildQuickActionInlinePreviewMetaSectionsModel` 抽出 inline preview meta 当前/轮次/视图分区顺序与空分区过滤规则 |
| quick action inline preview meta sections model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条分区顺序/空输入边界测试 |
| quick action inline preview header model helper | `buildQuickActionInlinePreviewHeaderModel` 抽出 inline preview card 顶部阶段/摘要文本规范化与空文本过滤规则 |
| quick action inline preview header model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条顶部文案/空输入边界测试 |
| quick action inline preview render model helper | `buildQuickActionInlinePreviewRenderModel` 组合 card/header/meta/controls/fieldRows/actions 纯模型，减少 `app.js` 手动串联 |
| quick action inline preview render model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条组合模型/缺失输入边界测试 |
| quick action inline preview panel model helper | `buildQuickActionInlinePreviewPanelModel` 组合 hint 与 card render model，统一 inline preview 可渲染判定 |
| quick action inline preview panel model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 panel 组合/缺失输入边界测试 |
| quick action inline preview primary field helper | `buildQuickActionInlinePreviewPanelModel` 支持从 `resolvedPreviewView.primaryFieldText` 推导主字段，收拢 `app.js` 的 `previewField` 前置判定 |
| quick action inline preview latest-view helper | 新增 `quickActionPreviewViewingLatest`，`buildQuickActionInlinePreviewRenderModel` 可从历史快照自动推导最新/历史轮 CTA 语义 |
| quick action inline preview panel model 追加测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 resolved preview 主字段推导 / 历史快照 latest-view 推导测试 |
| quick action inline preview hint DOM model helper | 新增 `buildQuickActionInlinePreviewHintDomModel`，把 inline preview hint 容器、分隔符、stage/field/round 节点 class/title/action 规格从 `app.js` 收到纯模型 |
| quick action inline preview hint DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 hint DOM 节点规格 / 无 round 规格测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview meta DOM model helper | 新增 `buildQuickActionInlinePreviewMetaDomModel`，把 inline preview meta 容器、label、current strip、pill class/dataset/action 规格从 `app.js` 收到纯模型 |
| quick action inline preview meta DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 meta 分区 DOM 规格 / 空分区与空 pill 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview controls DOM model helper | 新增 `buildQuickActionInlinePreviewControlsDomModel`，把 inline preview history / field-view 控制按钮容器、button type/title/dataset/action 规格从 `app.js` 收到纯模型 |
| quick action inline preview controls DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 controls DOM 分组规格 / 空按钮组与空标签过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview action DOM model helper | 新增 `buildQuickActionInlinePreviewActionDomModel`，把 inline preview snapshot/workflow CTA 容器、button type/title/aria/dataset/action id 规格从 `app.js` 收到纯模型 |
| quick action inline preview action DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 action DOM 按钮规格 / 空按钮组与空标签过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview field rows DOM model helper | 新增 `buildQuickActionInlinePreviewFieldRowsDomModel`，把 inline preview 字段列表容器、行、label/value 节点 class 与空值回退规格从 `app.js` 收到纯模型 |
| quick action inline preview field rows DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 field rows DOM 列表规格 / 空行与空标签过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview header DOM model helper | 新增 `buildQuickActionInlinePreviewHeaderDomModel`，把 inline preview 顶部阶段/摘要 line 的 class 与文本过滤规则从 `app.js` 收到纯模型 |
| quick action inline preview header DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 header DOM line 规格 / 空标题与空行过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview card DOM model helper | 新增 `buildQuickActionInlinePreviewCardDomModel`，把 inline preview card 容器 class、`actionIntensity` dataset 与 history/field-view 折叠 dataset flag 键从 `app.js` 收到纯模型 |
| quick action inline preview card DOM model 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 card DOM 容器/dataset flag 规格与空 intensity/default meta 测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview render model cardDom | `buildQuickActionInlinePreviewRenderModel` 现在直接输出 `cardDom`，把 `quickActionIntensity(action)` 与 card DOM 组合从 `app.js` 收回模型层 |
| quick action inline preview render model cardDom 测试 | `shell-quick-action-preview.test.mjs` 扩展 render model 组合测试，覆盖 `cardDom.dataset.actionIntensity` 和折叠 dataset flag |
| quick action inline preview meta action target helper | 新增 `quickActionInlinePreviewMetaActionTarget`，把 inline preview meta pill 的 history / field-view action target 校验与规范化从 `app.js` 收到纯模型 |
| quick action inline preview meta action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 history/field-view target 解析与无效 action 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview hint action target helper | 新增 `quickActionInlinePreviewHintActionTarget`，把 inline preview 顶部 hint 的 workflow / snapshot / history action target 校验与规范化从 `app.js` 收到纯模型 |
| quick action inline preview hint action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 workflow/snapshot/history target 解析与无效 action 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview control action target helper | 新增 `quickActionInlinePreviewControlActionTarget`，把 inline preview history / field-view 控制按钮的 action target 校验与规范化从 `app.js` 收到纯模型 |
| quick action inline preview control action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 controls target 解析与无效 action 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview action target helper | 新增 `quickActionInlinePreviewActionTarget`，把 inline preview 底部 snapshot/workflow CTA button id 到 action target 的解释从 `app.js` 收到纯模型 |
| quick action inline preview action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 snapshot/workflow CTA target 解析与无效 id 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action inline preview clickable DOM spec helper | 新增 `quickActionInlinePreviewClickableDomSpec`，把 inline preview hint/meta 可点击节点的 `is-clickable`、`tabIndex`、`role`、title/aria 规格从 `app.js` 收到纯模型 |
| quick action inline preview clickable DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条带 title/空 title 可访问性规格测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview key activation helper | 新增 `quickActionPreviewKeyActivates`，把 preview card 与 inline preview meta pill 的 Enter/Space 键盘激活判断从 `app.js` 收到纯模型 |
| quick action preview key activation 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 Enter/Space 激活与其他键过滤测试；同步 `fake-dom.mjs` import 替换映射并修复 import 顺序匹配 |
| quick action preview card DOM spec helper | 新增 `buildQuickActionPreviewCardDomSpec`，把普通 preview card 容器 class 与 `actionIntensity/quickAction/previewState` dataset 规格从 `app.js` 收到纯模型 |
| quick action preview card DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条容器 class/dataset 规格与空输入过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview clickable DOM spec helper | 新增 `quickActionPreviewClickableDomSpec`，并让 inline clickable helper 复用通用规格；普通 preview card meta pill 可点击规格不再手写在 `app.js` |
| quick action preview clickable DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条通用 preview 可点击规格测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card header DOM spec helper | 新增 `buildQuickActionPreviewCardHeaderDomSpec`，把普通 preview card header/heading/kicker/title class 与标题 fallback 规则从 `app.js` 收到纯模型 |
| quick action preview card header DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 header DOM 规格与 title fallback 测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview history controls DOM spec helper | 新增 `buildQuickActionPreviewHistoryControlsDomSpec`，把普通 preview card 历史快照按钮区 wrapper/label/button class、dataset、text/title 规格从 `app.js` 收到纯模型 |
| quick action preview history controls DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条多轮历史按钮区/单轮空返回测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview field-view controls DOM spec helper | 新增 `buildQuickActionPreviewFieldViewControlsDomSpec`，把普通 preview card 字段视图切换区 wrapper/button dataset、title/text 与 stage/snapshot 选择规则从 `app.js` 收到纯模型 |
| quick action preview field-view controls DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条字段视图按钮区/无切换返回 null 测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card pills DOM spec helper | 新增 `buildQuickActionPreviewCardPillsDomSpec`，把普通 preview card 当前/轮次/视图 pill 分组、dataset、title 与 action target 规格从 `app.js` 收到纯模型 |
| quick action preview card pills DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条完整 pill 分组/仅当前基础 pill 测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card sheet DOM spec helper | 新增 `buildQuickActionPreviewCardSheetDomSpec`，把普通 preview card 字段 sheet、row/label/value class 与 notes 拼接规格从 `app.js` 收到纯模型 |
| quick action preview card sheet DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条字段截断/notes 与无效字段过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card sheet render DOM spec helper | 新增 `buildQuickActionPreviewCardSheetRenderDomSpec`，把普通 preview card sheet wrapper、字段行子节点和 notes 子节点组合规格从 `app.js` 收到纯模型 |
| quick action preview card sheet render DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 sheet render children / 空 sheet 安全返回测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview control wrapper state helper | 新增 `buildQuickActionPreviewControlWrapperDomState`，把普通 preview card history/view 控制区 wrapper class、hidden 与 `aria-hidden` 合同从 `app.js` 收到纯模型 |
| quick action preview control wrapper state 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条折叠/展开 wrapper 状态测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview control panel DOM spec helper | 新增 `buildQuickActionPreviewControlPanelDomSpec`，把普通 preview card history/view 控制区 wrapper state、label、buttons 组合规格从 `app.js` 收到纯模型 |
| quick action preview control panel DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 history panel / field-view panel 组合规格测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card copy DOM spec helper | 新增 `buildQuickActionPreviewCardCopyDomSpec`，把普通 preview card summary/follow-up copy 优先级、class 与空值过滤从 `app.js` 收到纯模型 |
| quick action preview card copy DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 summary 优先/fallback 与空输入测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview summary line DOM spec helper | 新增 `buildQuickActionPreviewSummaryLineDomSpec`，把 preview summary 行 tag/class、lead、history chip、分隔符与 summary copy 节点规格从 `app.js` 收到纯模型 |
| quick action preview summary line DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条带前缀历史 chip/snapshot fallback 与空输入测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card pill action target helper | 新增 `quickActionPreviewCardPillActionTarget`，把普通 preview card pill 的 history / field-view action target 校验与规范化从 `app.js` 收到纯模型 |
| quick action preview card pill action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 history/field-view target 解析与无效 action 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card control action target helper | 新增 `quickActionPreviewCardControlActionTarget`，把普通 preview card 历史/字段视图控制按钮 target 校验与规范化从 `app.js` 收到纯模型 |
| quick action preview card control action target 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 history/field-view control target 解析与无效输入过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card pill sections DOM spec helper | 新增 `buildQuickActionPreviewCardPillSectionsDomSpec`，把普通 preview card pill 的 current/history/field-view 分区顺序与空分区过滤从 `app.js` 收到纯模型 |
| quick action preview card pill sections DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条分区顺序/空分区过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview card pill sections 残留清理 | 删除 `app.js` 中 `currentPills` 旧预渲染残留，避免 sections helper 接管后仍保留未定义/无效 DOM 路径 |
| quick action preview card pill sections 静态防回归 | `shell-pages-static.test.mjs` 新增约束，要求 `createQuickActionPreviewCard()` 只通过 `buildQuickActionPreviewCardPillSectionsRenderDomSpec` 渲染分区，禁止 `currentPills` / `pillSection ===` / 直接 action target 解释回流 |
| quick action preview card pill sections render DOM spec helper | 新增 `buildQuickActionPreviewCardPillSectionsRenderDomSpec`，把普通 preview card pill 分区和 history/field-view action target 规范化从 `app.js` 收到纯模型 |
| quick action preview card pill sections render DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条分区 target 规范化 / 无效 target 过滤测试；同步 `fake-dom.mjs` import 替换映射 |
| quick action preview control button DOM spec helper | 新增 `buildQuickActionPreviewControlButtonDomSpec`，把普通 preview card history/field-view 控制按钮 type/class/dataset/text/title/source 规格从 `app.js` 收到纯模型 |
| quick action preview control button DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条 history button / field-view button 规格测试；`quickActionPreviewCardControlActionTarget` 支持规范化后 `source`，`app.js` 点击逻辑不再回读 raw button spec |
| quick action preview control panel render DOM spec helper | 新增 `buildQuickActionPreviewControlPanelRenderDomSpec`，把普通 preview card history/view 控制区 wrapper/label/buttons 组合与按钮规范化从 `app.js` 收到纯模型 |
| quick action preview control panel render DOM spec 测试 | `shell-quick-action-preview.test.mjs` 新增 2 条控制区 render spec / 空按钮过滤测试；同步 `fake-dom.mjs` import 替换映射，修复临时 app 拷贝的 preview 模块导入 |
| Web Shell fake-dom import 基线修复 | `fake-dom.mjs` 对 `./shell-quick-action-preview.js` 增加模块级 URL 重写兜底，避免导入清单变化导致临时 app 从 `/tmp` 解析失败 |
| Web Shell fake-dom import 统一映射 | 新增 `APP_LOCAL_IMPORT_PATHS` / `rewriteAppLocalImports()`，统一重写 `app.js` 所有 `./*.js` 本地导入，并删除旧多行 `.replace()` 链与冗余 URL 常量 |
| fake-dom import 防回归测试 | 新增 `fake-dom-import-rewrite.test.mjs`，自动对比 `app.js` 当前本地 imports 与 fake-dom 映射表，并验证真实 app rewrite 后无相对本地导入残留 |
| quick action preview 控制文案契约同步 | `shell-quick-action-preview.test.mjs` 对齐 `historyLabel` 驱动的 snapshot 切换 title，保留“字段”语义 |
| pixel scene hotspot label CSS 合同修复 | 补齐 `styles.pixel-map.css` clear-mode 下 hotspot label 隐藏/hover/near-pointer/blank-click 可见规则，恢复 `shell-pages-static.test.mjs` 静态合同 |
| creative scene hotspot 真实交互修复 | `styles.pixel-map.css` 末尾恢复具体 hotspot `pointer-events: auto`，容器保持不拦截空白区；`shell-pages-static.test.mjs` 增加防回归，`verify-frontend-realness.mjs` 通过 |
| `app.js` 行数 | 9021 → 8955 → 8942 → 8936 → 8926 → 8925 → 8918 → 8904 → 8899 → 8881 → 8864 → 8828 → 8816 → 8804 → 8775 → 8771 → 8761 → 8759 → 8744 → 8672 → 8681 → 8671 → 8677 → 8679 → 8674 → 8684 → 8656 → 8651 → 8649 → 8648 → 8645 → 8632 → 8638 → 8641 → 8651 → 8651 → 8647 → 8649 → 8651 → 8730 → 8732 → 8737 → 8738 → 8739 → 8742 → 8743 → 8744 → 8791 → 8752 → 8755 → 8764 → 8768 → 8747 → 8749 → 8752 → 8743 → 8740 → 8742 → 8746 → 8752 → 8747 → 8746 |
| Web Shell 测试 | 691 passed / 0 failed；layout / realness passed |

## Operating Rules

- 主线优先级：后端 gateway 合同 > H5 IM 主路径 > TUI parity > admin 精简 > 文档同步。
- 每轮只处理 1-2 个最高优先级任务。
- 每轮必须跑相关测试；如果测试失败，继续修到通过。
- 不提交 git，不删除重要文件，不改无关项目。
- 遇到高风险操作、需求冲突、产品取舍、外部账号/付费/删除/提交时停下来问用户。
- 每轮结束写入项目根目录的 longrun 记录文件：进度、改动文件、测试结果、下一轮目标。
- 如果 CC 正在做 H5 前端，避免并发改同一批 H5 文件；优先做 Rust gateway/TUI 或只做验收。

## 2026-05-25 四阶段后端补齐完成摘要

### Phase 1-4 已完成项

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| Phase 1 | 盘点后端结构，建立测试基线 | 完成 |
| Phase 2 | 补齐登录、身份、房间、消息、在线、未读、会话切换 | 完成 |
| Phase 3 | 居民目录搜索、在线状态、头像、私聊入口、DM 走 Gateway | 完成 |
| Phase 4 | 管理后台 API — 居民、房间、消息、系统状态的读侧端点 | 完成 |

### 新增/改动的后端能力

| 能力 | 端点/合同 | 测试 |
| --- | --- | --- |
| 在线状态 heartbeat | `POST /v1/shell/presence` | presence_http_endpoint_roundtrips |
| 未读标记清零 | `POST /v1/shell/read` | mark_read_http_endpoint_resets_unread |
| 居民搜索 | `GET /v1/residents?q=` | resident_endpoint_supports_search_query |
| 居民目录 enriched | `GET /v1/residents` (含 online/last_seen/avatar) | presence_appears_in_enriched_resident_directory |
| ShellState 未读字段 | `ShellRoomState.unread_count` | shell_state_includes_unread_count |
| 管理后台系统摘要 | `GET /v1/admin/summary` | admin_summary_endpoint_returns_counts_and_uptime |
| 消息发布后增量未读 | `publish_message()` 自动递增 | unread_increments_after_message_publish |

### 测试基线

- `cargo test -p lobster-waku-gateway`: 122 passed / 0 failed
- `cargo test --release -p lobster-waku-gateway`: 122 passed / 0 failed
- `node --test apps/lobster-web-shell/test/*.mjs`: 177 passed / 0 failed
- 总计: 299 测试全部通过

## Current Priority Queue

### P0 / P1

1. H5 IM 主路径真实验收
   - 验证 `index.html` 与 `creative.html` 双端互发。
   - 要求：发送后输入框清空；己方右侧、对方左侧；每条头像可见；pending echo 被 committed copy 替换；无重复闪现/撤回；失败状态来自真实 gateway。
   - 如果根因是 gateway 合同，修后端；如果是 UI 状态机，记录给 CC 或在未冲突时修 H5。

2. Gateway 合同继续加固
   - 检查 `/v1/shell/message`、`/v1/shell/events`、`/v1/shell/state` 的双端真实 IM 合同是否还有未覆盖边界。
   - 优先补黑盒测试，少改生产逻辑；只有测试暴露问题时再修实现。

3. TUI parity
   - 保持 `/help`、`/status`、`/refresh` 本地反馈不写 gateway。
   - 补齐 gateway recall/edit/send 状态投影端到端测试。
   - 不允许本地伪造成功态。

4. Admin 精简验收
   - 默认首屏只保留：左侧可收起分类导航、中间当前会话、右侧当前工具说明。
   - 高级功能藏到左侧分类选项卡里；disabled 必须有原因。
   - 不继续堆一屏表单墙。
   - `GET /v1/admin/summary` 端点已可用，前端对接待推进。

5. 文档同步
   - `creative.html` 是居民/住宅私聊主入口。
   - `user.html` 只保留 query-preserving 兼容跳转。
   - Gateway 是唯一合同真源。

## 2026-05-27 CC DS v4 Pro + Flash 混合执行摘要

### 本轮完成

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| Phase 0 | 建立基线，外盘备份 | 完成 |
| Phase 1 | 安全拆 app.js — shell-message-render.js (57行, 7个纯函数) | 完成 |
| Phase 2 | admin-ds 补加载/空/错误/刷新状态 | 完成 |
| Phase 3 | 补齐测试：shell-message-render 单测 + admin-ds 状态测试 | 完成 |
| Phase 4 | 技术债整理与文档同步 | 进行中 |

### 改动文件

| 文件 | 操作 | 行数变化 |
|------|------|---------|
| `shell-message-render.js` | **新增** | 57 行 |
| `app.js` | 删除 7 个函数定义 + 新增 import | 9879 → 9847 (−32) |
| `admin-ds.js` | 补 renderEmptyRow/setSectionLoading + 5 个表格空状态 + 加载/错误态改善 | 1172 → 1212 (+40) |
| `test/shell-message-render.test.mjs` | **新增** | 25 个测试 |
| `test/admin-ds-static.test.mjs` | 新增 5 个状态测试 | +~70 行 |
| `test/fake-dom.mjs` | 新增 shell-message-render.js URL 解析 + import 替换 | +~20 行 |
| `docs/app-js-split-audit.md` | 更新：子 agent 审查发现 + 下一轮优先级 | +~120 行 |

### 测试基线

- `npm test`: 338 passed / 0 failed (+30 vs 上次基线)
- `npm run smoke:dual-browser`: passed
- JS 语法检查：app.js, admin-ds.js, 13 个 shell-*.js 全部通过
- Backup: `/Volumes/AJW-Data/Backups/lobster-chat-cc-longrun-20260527-1030/`

### 真实功能新增

- admin-ds.js 全部 5 个表格现在有**空状态**占位行（含搜索无匹配提示）
- admin-ds.js 居民/房间/消息模块有**加载状态**（opacity + data-loading 属性）
- admin-ds.js 网关读取有**部分失败检测**（Promise.allSettled + rejected 处理）
- shell-message-render.js 提供 7 个可复用纯函数：messageStableId, isSystemSender, messageAvatarTone, messageThreadKind, messageRoleLabel, formatDateTime, escapeHtml

### 仍未完成

1. admin-ds sysconfig 已接入真实写操作，其余 12 处仍 disabled（需后端 Gateway 写接口）
2. admin-ds 邀请码/日志模块数据仍来自 Mock（无对应 Gateway 端点）
3. admin-ds 房间/邀请码表格尚无分页
4. app.js 仍有 9,847 行，重复逻辑未系统清理（详见审计文档第 7 节）

## 2026-05-28 第三轮摘要

### 本轮完成

- **Phase 1**: admin-ds `buildContextMessages` 假数据替换（上一轮）
- **Phase 2**: admin-ds 居民/消息/日志三表前端分页（上一轮+本轮修复）
- **Phase 3**: admin-ds 第一个真实写操作 — `POST /v1/admin/config`
  - 新增 `fetchGatewayJsonPost(path, body)` helper（安全 POST 封装）
  - 新增 `loadSysConfig()` — GET /v1/admin/config 加载配置
  - 新增 `renderSysConfigEditor(config)` — 动态渲染键值编辑 UI
  - 新增 `saveSysConfigItem(key, value, btnEl)` — POST 保存单项配置
  - 新增 `addSysConfigItem()` — 添加新参数到 Gateway
  - `switchModule('sysconfig')` 自动触发加载
- 修复 `renderPagination` 中 `el()` 调用 `className` → `class` 键名 bug
- 修复 `renderResidents` IIFE 闭包 `filtered[i]` → `residentPage[i]` bug

### 改动文件

| 文件 | 操作 | 行数变化 |
|------|------|---------|
| admin-ds.js | 增强 | 1299→~1380 |
| admin-ds.html | 增强 | sysconfig 动态编辑器 |
| verify-frontend-realness.mjs | 更新 | 分页校验规则 |

### 后端合同已验证

- `GET /v1/admin/config` → `HashMap<String, String>` JSON
- `POST /v1/admin/config` → 接受 `{"config": {"key": "value"}}`，返回 `{"ok": true}`
- `admin_set_config()` 支持增量更新（merge 而非 replace）
- 移除 `.ds-page-btn` 的 blanket `markUnavailableButton`
- `verify-frontend-realness.mjs` 分页校验规则同步更新

### 测试基线

- `npm test`: 338 passed / 0 failed
- layout: all OK
- realness: passed

### 下一轮建议

按优先级：
1. **app.js 低风险清理** — quickAction switch 函数提取（~180行，7个纯函数）
2. **admin-ds 第二个写操作** — POST /v1/admin/residents/ban 打通管理链路
3. **admin-ds 房间/邀请码表格分页** — 补全剩余两表
4. **shell-room-rail.js 魔数整治** — roomGroupBlueprints 权重值注释 + joinOrFallback 统一
5. **多页面 left-rail 一致性** — creative/index/unified/world-square 统一宽度/框架

## 2026-05-31 / 06-01 DS v4 Pro 执行摘要

### 本轮完成

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| P0-1 | 建立 git 安全快照和备份 | 完成 |
| P0-2 | 修正本地预览 gateway 启动 (restart-gateway.sh) | 完成 |
| P0-3 | 修复 H5 Enter 发送策略 (统一 PC/Mobile: Enter=发送, Shift+Enter=换行) | 完成 |
| P1-1 | admin-ds 消息审核写操作闭环 — POST /v1/admin/messages/moderate | **完成** |

### P1-1 详情: 消息审核写操作闭环

**后端 (上一轮已完成):**
- `gateway_models.rs`: 新增 `AdminModerateMessageRequest` struct
- `core_runtime.rs`: 新增 `admin_moderate_message()` 方法 + `message_moderation: HashMap<String, String>`
- `http_write_routes.rs`: 新增 `handle_post_admin_moderate_message()` handler
- `http_router.rs`: 注册 `POST /v1/admin/messages/moderate` 路由
- `gateway_tests.rs`: 3 个测试 (roundtrip / invalid action / nonexistent message)

**前端 (本轮完成):**
- `admin-ds.js` `normalizeGatewayMessages()`: 保留 `message_id` 和 `conversation_id` 字段
- `admin-ds.js`: 新增 `moderateMessage()` helper 和 `refreshCurrentMessageView()` helper
- `admin-ds.js`: "通过"/"屏蔽" 按钮接入真实 POST (含 loading 态、失败反馈、成功后刷新)
- `admin-ds.js`: 两处 "标记已处理" 按钮接入真实 POST
- `admin-ds-data.js`: 新增 `approved`/`handled` 状态标签和样式

### 改动文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `admin-ds.js` | 修改 | normalizeGatewayMessages 保留 ID 字段 + moderateMessage helper + 6 个按钮 wiring |
| `admin-ds-data.js` | 修改 | msgStatusTag/msgStatusText 增加 approved/handled 条目 |
| `restart-gateway.sh` | **新增** | Gateway 一键构建+重启+健康检查脚本 |

### 测试基线

- `cargo test -p lobster-waku-gateway`: **176 passed / 0 failed**
- `npm test` (前端): **538 passed / 0 failed**
- JS 语法检查: admin-ds.js, admin-ds-data.js 全部通过
- E2E 验证: `POST /v1/admin/messages/moderate` 三种 action + 两种错误情况全部通过

### P1-2: app.js 安全拆分 (2026-06-01)

从 app.js (9119行) 提取 3 个纯数据变换函数 → `shell-state-normalize.js` (107行):

| 函数 | 说明 |
|------|------|
| `contractConversationMap(payload)` | gateway payload → Map<conversation_id, room> |
| `mergeRoomWithContract(room, contract)` | room 原始数据 + contract 默认值合并 |
| `synthesizeRoomsFromContracts(payload)` | 组合上述两函数，生成完整 room 数组 |

- app.js: 9119 → 9020 (-99)
- 新增 `test/shell-state-normalize.test.mjs`: 9 个测试
- 累计 app.js 缩减: 9847 → 9020 (-827)
- 新增独立模块: `shell-state-normalize.js` (107行, 3 函数, 9 测试)

### 仍未完成

1. admin-ds 居民封禁/解禁、房间冻结/解冻仍 disabled（后端接口已有，前端未接入）
2. admin-ds 邀请码/日志模块数据仍来自 Mock（无对应 Gateway 端点）
3. app.js 仍有 ~9020 行，`roomMatchesSearch` (64行) 可提取但需先提取依赖函数
4. 消息审核状态仅存于 GatewayRuntime 内存（HashMap），不持久化，重启后丢失

## 前端审计结果 (2026-05-25)

- H5 `app.js` 的 `state` 变量初始化为 `structuredClone(SAMPLE_STATE)` 兜底，gateway 连接后全部来自 `normalizeShellState()`。
- `activeRoomId` 是 UI 视图状态，跟踪用户选中哪个房间，不是 canonical 数据。
- 无直接 `state.rooms.push()` 或 `state.* = ...` 绕过 gateway 的路径。
- `admin-ds.js` 只有临时渲染数组和显示标签，无本地 canonical 状态。

## Standard Test Commands

```bash
cargo test -p lobster-waku-gateway
cargo test -p lobster-tui
node --test apps/lobster-web-shell/test/*.mjs
./scripts/smoke-web-shell.sh
```

## Browser Acceptance Targets

```text
http://127.0.0.1:18081/index.html?gateway=http://127.0.0.1:8787&identity=qa-a
http://127.0.0.1:18081/creative.html?gateway=http://127.0.0.1:8787&identity=qa-b
http://127.0.0.1:18081/admin.html?gateway=http://127.0.0.1:8787&qa=manual
http://127.0.0.1:18081/unified.html?gateway=http://127.0.0.1:8787&qa=manual
http://127.0.0.1:18081/world-square.html
```

## 2026-06-11 Codex 技术债补充: start-terminal 预构建入口收口

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯契约 | 完成 | `scripts/test_start_terminal_shell_unit.py` 增加 `SKIP_BUILD`、`GATEWAY_BIN`、`TUI_BIN`、binary 可执行检查和 exec 路径断言 |
| 启动脚本 | 完成 | `scripts/start-terminal.sh` 支持 `SKIP_BUILD=1` 走预构建 gateway/TUI，不再无条件要求 cargo |
| Gateway 启动 | 完成 | 非复用现有健康端点时先按需构建 `lobster-waku-gateway`，再通过 `GATEWAY_BIN` 启动 |
| TUI 启动 | 完成 | 按需构建 `lobster-tui`，最终通过 `TUI_BIN` 启动，缺失时报明确路径 |

### 验证

```bash
python3 scripts/test_start_terminal_shell_unit.py
python3 scripts/test_start_terminal_unit.py
bash -n scripts/start-terminal.sh
python3 scripts/test_package_release_unit.py && python3 scripts/test_scripts_quick_unit_coverage.py && python3 scripts/test_smoke_release_gate_unit.py && python3 scripts/test_smoke_provider_federation_unit.py && python3 scripts/test_smoke_web_dual_browser_unit.py && python3 scripts/test_smoke_resident_mainline_unit.py && python3 scripts/test_smoke_cli_channel_unit.py && python3 scripts/test_smoke_auth_registration_unit.py && python3 scripts/test_smoke_shell_dual_http_unit.py && python3 scripts/test_smoke_shell_direct_http_unit.py && python3 scripts/test_smoke_web_shell_unit.py && python3 scripts/test_install_server_unit.py && python3 scripts/test_preview_server_unit.py && python3 scripts/test_start_terminal_shell_unit.py && python3 scripts/test_audit_web_assets_unit.py && python3 scripts/test_lobster_device_id_unit.py && python3 scripts/test_start_web_preview_unit.py && python3 scripts/test_restart_gateway_unit.py && python3 scripts/test_preflight_unit.py && python3 scripts/test_smoke_public_ingress_unit.py && python3 scripts/test_smoke_install_layout_unit.py && python3 scripts/test_start_terminal_unit.py && python3 scripts/test_makefile_unit.py
bash -n scripts/package-release.sh scripts/smoke-provider-federation.sh scripts/smoke-release-gate.sh scripts/smoke-resident-mainline.sh scripts/smoke-cli-channel.sh scripts/smoke-auth-registration.sh scripts/smoke-shell-dual-http.sh scripts/smoke-shell-direct-http.sh scripts/install-server.sh scripts/smoke-web-shell.sh scripts/start-terminal.sh scripts/audit-web-assets.sh scripts/lobster-device-id.sh scripts/restart-gateway.sh scripts/preflight.sh scripts/smoke-public-ingress.sh scripts/smoke-install-layout.sh
zsh -n scripts/start-web-preview.sh
node --check scripts/preview-server.mjs
node --check scripts/smoke-web-dual-browser.mjs
git diff --check
```

## 2026-06-17 Codex 技术债推进: web-shell app.js 长函数与纯规格抽取

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| app.js 长函数清零 | 完成 | 拆分 `ensureWorkspaceChrome()`、`ensureUserSceneChrome()`、`syncRoomStageCanvas()`，并继续拆 `loadWorldState()` / `main()`；当前全部 `function`（含 async）扫描无超过 50 行的函数 |
| workspace chrome 拆分 | 完成 | 工作区 nav、用户搜索、room toolbar、composer 辅助层、caretaker chrome 分离，并新增静态护栏 |
| user scene chrome 拆分 | 完成 | 房间侧栏、舞台 canvas、人物 canvas、chat-detail 面板拆成独立 helper，不改场景热点/图层交互合同 |
| room stage canvas 拆分 | 完成 | 默认住宅画布、用户房间画布、note 更新与 visual 构建分离 |
| world state payload 纯模型 | 完成 | `governanceFromWorldSnapshotBundle()` / `governanceFromWorldApiPayload()` 移入 `shell-state-normalize.js`；`loadWorldState()` 只保留请求与赋值，新增 3 条单测和静态防回流护栏 |
| main 启动编排拆分 | 完成 | `initializeLocalShellState()`、`loadInitialRuntimeState()`、`bindSceneEditorLink()`、`renderInitialShell()` 接管启动阶段；`main()` 不再直接拼 scene-editor URL 或堆本地状态加载细节 |
| composer placeholder 纯函数 | 完成 | `composerPlaceholderForState()` 移入 `shell-room-render.js`，新增 6 条单测；`app.js` 只收集运行态并委托 |
| message owner action spec | 完成 | `messageOwnerActionSpecs()` 移入 `shell-message-render.js`，新增 edit/recall 可见性单测；DOM 事件绑定仍留在 `app.js` |
| chat detail runtime rows 纯模型 | 完成 | `chatRuntimeDetailModelForState()` 移入 `shell-room-render.js`，把聊天详情运行状态行、quick action 行、provider/输入/管家/错误行顺序下沉；`app.js` 只负责 detail row DOM 与预览卡交互 |
| timeline committed render items | 完成 | `timelineCommittedMessageRenderItems()` 移入 `shell-message-render.js`，把已提交消息的未读/日期 divider 与 message row 顺序下沉；`app.js` 只按 item 类型创建透明 DOM 节点 |
| world directory 纯规格 | 完成 | 新增 `shell-governance-render.js`，把世界目录空状态和城市卡文案/class 规格下沉；`renderWorldDirectory()` 只负责 DOM 创建和列表挂载 |
| mirror sources 纯规格 | 完成 | `mirrorSourcesEmptyStateText()` / `mirrorSourceCardModel()` 下沉世界镜像源空状态、状态行、计数行和最近快照文案；`renderMirrorSources()` 只负责 DOM 创建和列表挂载 |
| world square notice 纯规格 | 完成 | `worldSquareEmptyStateText()` / `worldSquareNoticeCardModel()` 下沉世界广场空状态、标题/meta、正文、标签/时间文案；`renderWorldSquare()` 只负责 DOM 创建和列表挂载 |
| world safety mirror 纯规格 | 完成 | `worldSafetyEmptyStateText()` / `worldSafetyMirrorCardModel()` 下沉世界安全空状态、镜像城市数量、信任状态列表和治理员文案；`renderWorldSafety()` 继续只做列表组合 |
| world safety advisory 纯规格 | 完成 | `worldSafetyAdvisoryEmptyStateText()` / `worldSafetyAdvisoryCardModel()` 下沉世界安全通告空态、动作、对象类型和发布时间文案；`appendWorldSafetyAdvisoryCards()` 只负责分支和挂载 |
| world safety summary/detail 纯规格 | 完成 | `worldSafetySanctionSummaryCardModel()` / `worldSafetyReportSummaryCardModel()` / `worldSafetySanctionCardModel()` / `worldSafetyReportCardModel()` 下沉制裁/举报摘要与明细文案；`renderWorldSafety()` 和 append helpers 只做列表组合 |
| resident directory 纯规格 | 完成 | `residentDirectoryEmptyStateText()` / `residentDirectoryCardModel()` 下沉居民目录空态、标题/slug、已加入/待审批城市与身份行文案；`app.js` 只保留 DOM 创建、私聊按钮和当前身份判断 |
| caretaker panel 纯规格 | 完成 | 新增 `shell-caretaker-panel.js`，下沉非居民页管家面板标题、资料、消息、规则和状态条 items；`app.js` 只负责 DOM 创建和当前房间标题注入 |
| governance offline/header 纯规格 | 完成 | `governanceOfflineStateModel()` / `governanceWorldHeaderModel()` / `governanceEmptyCityStateModel()` 下沉治理离线态、世界 header 摘要和空城市列表文案；`app.js` 只负责清 DOM 与挂载 |
| governance city card base 纯规格 | 完成 | `governanceCityCardBaseModel()` 下沉城市卡基础 class、标题/slug、简介、成员状态与公开发现/入城审批文案；`app.js` 只负责创建 DOM 和注入 `humanMembership()` |
| governance city room list 纯规格 | 完成 | `governanceCityRoomListModel()` 下沉公共房间标题、容器/行 class、冻结标签、打开/冻结按钮文案和冻结权限判定；`app.js` 只保留按钮事件绑定与 Gateway 调用 |
| governance member lists 纯规格 | 完成 | `governancePendingMemberListModel()` / `governanceActiveMemberListModel()` 下沉待审批/活跃居民列表标题、容器/行 class、批准/执事按钮文案和权限判定；`app.js` 只保留 approve/steward 事件绑定与 Gateway 调用 |
| governance city actions 纯规格 | 完成 | `governanceCityActionsModel()` 下沉加入、等待审批、打开大厅、新建房间动作的 class/文案/可见性与 lobby fallback；`app.js` 只保留输入聚焦、状态提示和 Gateway 调用 |
| governance federation policy 纯规格 | 完成 | `governanceFederationPolicyControlsModel()` 下沉联邦策略标题、选项行、当前/应用按钮状态和权限判定；`app.js` 只保留 `submitFederationPolicy()` 事件绑定 |
| room stage 投影纯函数 | 完成 | 新增 `shell-room-stage.js`，下沉舞台摘要、画像摘要/标题、画像 chips、私宅/公共频道投影文案；新增 5 条单测和静态防回流护栏 |
| composer context 纯模型 | 完成 | 新增 `composerContextItemsForState()`，把 composer context 文案、状态和 tone 规则移入 `shell-room-render.js`；`app.js` 与既有 `shell-composer.js` 均改为复用同一 helper，新增 4 条单测和静态防回流护栏 |
| composer hero 纯模型 | 完成 | 新增 `composerHeroModelForState()`，统一 hero variant/kicker/title/note/chips；`app.js` 和既有 `shell-composer.js` 均只负责 DOM 创建，新增 3 条单测和静态防回流护栏 |
| fake-dom import 映射 | 完成 | 全量测试红灯暴露 `app.js` 新增 `shell-room-stage.js` import 后 fake-dom 临时模块重写漏映射；已补 `APP_LOCAL_IMPORT_PATHS`，相关 import rewrite / shell init 测试转绿 |
| scene-editor token 回归 | 完成 | realness 暴露 hub 页 query-only gateway 策略导致 editor href 不带 token；新增 `sceneEditorGatewayUrl()`，只让编辑器入口 fallback 到 remembered gateway，不改变 hub 消息网关策略 |
| message search DOM 纯规格 | 完成 | `messageSearchBarDomSpec()` / `searchResultItemDomSpec()` / `searchEmptyStateDomSpec()` 下沉搜索栏与搜索结果节点规格；`app.js` 搜索 UI 改为 `createElement` / `textContent` / `replaceChildren()`，移除搜索路径 `innerHTML` sink |
| message search request 纯模型 | 完成 | `messageSearchRequestModel()` 下沉空查询/缺网关/缺房间/缺居民身份 guard、query trim、room/resident/query/limit 编码与 `/v1/shell/messages/search` URL 组合；控制器以请求 options 透传 Bearer |
| message search target 匹配纯函数 | 完成 | `messageSearchRowMatchesId()` 下沉搜索结果跳转的 `message_id` 精确匹配；`searchResultItemDomSpec()` 保留可字符串化 message_id（如 `0`）；`app.js` 改为扫描 `[data-message-id]` 候选后比较 `dataset.messageId`，不再把 gateway/search 返回的 messageId 拼进 CSS selector |

### 当前指标

| 指标 | 当前值 |
| --- | ---: |
| `apps/lobster-web-shell/app.js` 行数 | 9733 |
| `app.js` >50 行函数（函数体括号范围扫描，含 async） | 0 |
| `shell-pages-static.test.mjs` | 151 passed |
| `shell-caretaker-panel.test.mjs` | 2 passed |
| `shell-message-search.test.mjs` | 14 passed |
| `shell-message-render.test.mjs` | 46 passed |
| `shell-room-render.test.mjs` | 51 passed |
| `shell-governance-render.test.mjs` | 39 passed |
| `shell-state-normalize.test.mjs` | 12 passed |
| web-shell 全量测试 | 1007 unit passed / 0 failed，layout passed，realness passed |

### 验证

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat/apps/lobster-web-shell
node --test test/shell-room-render.test.mjs
node --test test/shell-message-render.test.mjs
node --test test/shell-caretaker-panel.test.mjs
node --test test/shell-message-search.test.mjs
node --test test/shell-governance-render.test.mjs
node --test test/shell-pages-static.test.mjs
node --test test/shell-state-normalize.test.mjs test/shell-pages-static.test.mjs test/fake-dom-import-rewrite.test.mjs
node --test test/shell-composer.test.mjs
node --test test/shell-room-stage.test.mjs
node --check app.js
node --check shell-message-render.js
node --check shell-room-render.js
node --check shell-caretaker-panel.js
node --check shell-governance-render.js
node --check shell-state-normalize.js
for f in app.js shell-*.js composer-state.js pretext-stage.js; do node --check "$f" || exit 1; done
node verify-frontend-realness.mjs
npm test
git diff --check
```

### 下一步建议

1. 继续按 TDD 小步拆 `app.js`，优先选不碰 CC 交互改动的纯规格/文案层：governance list DOM specs、conversation overview 非用户状态/动作 specs、thread status rail 后续组合 specs。
2. 暂缓直接外提 `roomAudienceLabel()` / `roomSummaryLine()` 这组函数；它们在 `app.js` 里仍依赖 governance/publicRoom 实时状态，需先设计参数化边界再动。
3. CC/DS 若推进场景交互，必须保留 realness 的 scene-editor owner-only/token 护栏和三层/四层热点结构测试。

## 2026-06-16 Codex 复盘: CC 近期推进核验与技术债护栏恢复

### 读取来源

| 来源 | 结论 |
| --- | --- |
| 真实代码 | `/Volumes/AJW-Data/Projects/lobster-chat` 当前仍有较多未提交改动，包含 CC/DS 推进的 admin、世界广场、像素资产和主题实验；不能只按提交记录判断完成度 |
| CC 近期记录 | `/Users/rsaga/.ai-checkpoints/claude-code-sessions/project-state.md` 与 2026-06-12 session 记录显示：Gateway 244、TUI 225、CLI 50、web-shell 800 tests 作为近期绿线；CSS 已做 `styles.user-shell.css` 抽取，`styles.css` 目标为 5752 行 |
| 当前偏差 | 本轮进入时 `npm test` 表面可过，但真实覆盖已漂移：admin-ds 写操作测试被削弱，`styles.css` 被回灌到 17852 行，页面丢失 2026-06-12 拆分样式引用，creative 场景暖色遮罩被加回 |

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| admin-ds 写操作护栏 | 完成 | 恢复/补强 ban/unban/freeze/unfreeze 运行时测试，覆盖 POST body、prompt cancel、空 ID、无 gateway、JSON POST helper |
| 受控写接口静态合同 | 完成 | 将旧“只读后台”测试修正为“读取 gateway projection + 所有写操作走统一 `fetchGatewayJsonPost` helper”，防止散落 raw POST/DELETE/PUT/PATCH |
| CSS split 防回归 | 完成 | `index.html`、`admin.html`、`creative.html` 补回 `styles.base.css` / `styles.scene.css` / `styles.chat.css` / `styles.user-shell.css` 引用断言 |
| `styles.css` 回退修复 | 完成 | 将 `styles.css` 从 17852 行恢复到已提交拆分基线 5752 行；回退前备份在 `/Volumes/AJW-Data/Backups/lobster-chat-style-regression-20260616/styles.css.before-restore` |
| 场景美术护栏 | 完成 | 移除 creative day/night 之上的暖色径向遮罩，并将测试锁定到拆分后的 `styles.scene.css` 真实位置 |
| rail 宽度覆盖 | 完成 | 修正 `styles.scene.css` 后置规则把 public-square rail 固定为 `220px` 的覆盖，统一回 `var(--im-scene-rail-width, 220px)` |
| app.js 身份 helper 去重 | 完成 | 恢复 `shell-identity.js` 复用，移除 `app.js` 内联的 visitor/scoped/route/display helper；`fake-dom` import rewrite 白名单同步补回 |

### 验证

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat/apps/lobster-web-shell
node --test test/admin-ds-runtime.test.mjs test/admin-ds-static.test.mjs test/shell-pages-static.test.mjs
# 101 passed / 0 failed

npm test
# unit: 803 passed / 0 failed
# layout: verify-scene-layout.mjs passed
# realness: verify-frontend-realness.mjs passed
```

### 当前真实进度百分比

| 模块 | 当前估算 | 依据 |
| --- | ---: | --- |
| P0 单城邦中心化 IM 真闭环 | 98% | 身份、shell state、send/edit/recall/export、presence/read、admin summary 已有测试和 smoke 基线；剩余主要是上线环境复验 |
| P1 空间房间与交互完善 | 82% | `image_layer` / `hotspot_layer` 和 H5 渲染已成型，场景编辑器存在；仍需把编辑器 UX、移动端交互和 gateway 保存路径继续压实 |
| P2 后台与运维可用 | 91% | admin-ds 已接入多类真实读写端点与失败反馈；邀请码/部分日志/高级配置仍有 mock 或待接入边界 |
| P3 技术债与工程鲁棒性 | 74% | Gateway/TUI/CLI/release gate 已大量收口，CSS 拆分恢复；但 `app.js` 仍 8892 行，`admin-ds.js` 2951 行，前端模块边界仍是最大剩余债 |
| P4 TUI/CLI parity | 95% | TUI 225、CLI 50 的近期基线存在，send/edit/recall/export parity 已补；后续以 release smoke 和真实运行复验为主 |
| P5 真实 transport / 加密 / 跨城 | 15% | 仍属后置，不应阻塞当前单城 IM MVP |

### 下一步开发计划

1. 先把本轮 web-shell 护栏保持绿色，不再让 CC/DS 在 UI/素材推进时回灌拆分前 CSS 或删除测试覆盖。
2. 继续按 TDD 拆 `app.js`，优先提取低风险纯函数：room projection/search、composer 状态、message action、scene hotspot runtime；每次只拆一个边界并跑 `node --test test/*.test.mjs`。
3. admin-ds 继续补真实后端端点的写操作闭环；没有 Gateway 端点的模块只做禁用原因/只读投影，不做假成功态。
4. 对世界广场、主题实验、像素资产改动补最小静态/realness 测试，防止视觉推进破坏 IM 主入口和 day/night 美术约束。
5. 每轮 CC/DS 合并前至少跑：`npm test`、对应 Gateway/Rust 聚焦测试、`git diff --check`；大体量素材和备份继续放 `/Volumes/AJW-Data`。

### 给 CC / DS v4 的继续推进提示词

```text
你接手 /Volumes/AJW-Data/Projects/lobster-chat，先读 /Users/rsaga/.codex/memories/ACTIVE-im.md、docs/ACTIVE_WORK_QUEUE.md、docs/IMPLEMENTATION_PHASES.md、docs/DEVELOPMENT_BLUEPRINT.md。当前主线是单城邦中心化 IM，gateway 合同是唯一真源，H5 是主入口，TUI/CLI 做同合同 parity。

不要回灌拆分前 CSS，不要删除测试让 npm test 变绿。web-shell 当前基线：npm test 必须通过，unit 约 802 passed，layout 和 realness 也必须过；styles.css 应保持约 5752 行，base/scene/chat/user-shell 拆分样式必须被 index/admin/creative 正确引用。像素日景禁止暖黄/奶油/金色罩层，day/night 走 body[data-time-of-day] + PNG 直切。

优先做非技术债业务时：只接已有 Gateway 端点，admin-ds 写操作统一走 fetchGatewayJsonPost，失败要有反馈，待接入功能必须 disabled + reason，不能假成功。推进世界广场或主题 UI 时必须保护 IM 主入口、居民房间、主城、后台三条路径。

若继续技术债：按 TDD 小步拆 app.js。下一块建议提取 room projection/search 或 composer 状态纯函数，新增/移动测试后先看红灯，再实现，最后跑 npm test。不要同时改多个边界，不要动无关素材和生成包。
```

### 给 CC/DS 的后续建议

优先继续处理不碰业务体验的大块技术债：

1. 把剩余 smoke 脚本的 `SKIP_BUILD` / `*_BIN` / artifact 行为补成统一契约测试，避免 release gate 在无 Rust 工具链环境中误触发构建。
2. 对 `apps/lobster-web-shell/app.js` 做低风险纯函数提取，每次只拆一个函数并配套 `node --test`。
3. admin-ds 继续推进非技术债业务模块时，优先接入已有后端写接口，暂缓没有 Gateway 端点的 mock 模块。
4. 每轮改动后固定跑快速脚本单测、语法检查、`git diff --check`，不要生成或提交 `dist/` 和 `apps/lobster-web-shell/generated/*.json`。

## 2026-06-11 Codex 技术债补充: install-server 依赖顺序收口

## 2026-07-16 app.js 会话摘要投影继续收口

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| room digest 统计投影 | 完成 | 新增 `roomDigestMetricsSpec`，将未读、草稿、私信/频道/系统数量、待跟进和管家提醒统计从 `app.js` 下沉到 `shell-room-rail.js` |
| 主入口边界 | 完成 | `roomDigestMetrics()` 仅保留当前 active room 的运行时绑定，统计规则由纯模块统一提供 |
| TDD 护栏 | 完成 | 新增纯函数聚合测试和 app.js 静态委托测试，先红后绿 |

### 验证

```bash
cd /Volumes/AJW-Data/Projects/lobster-chat/apps/lobster-web-shell
node --test test/shell-room-rail.test.mjs test/shell-pages-static.test.mjs
# 306 passed / 0 failed
```

本轮不改变会话摘要文案或 DOM 结构；生产域名、TLS、真实邮件 OTP 和双端外部验收仍是上线前唯一未在当前本地环境闭环的事项。

### 本轮完成

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 红灯契约 | 完成 | `scripts/test_install_server_unit.py` 锁定 `WEB_ARTIFACT` 路径校验必须早于 Rust 工具链检查，源码构建分支必须在 `cargo build` 前调用 `ensure_modern_rust` |
| 安装脚本 | 完成 | `scripts/install-server.sh` 不再在 gateway artifact 路径外提前触发 Rust 工具链；只有真正从源码构建 gateway 时才执行 `ensure_modern_rust` |
| 失败顺序 | 完成 | 缺失 `WEB_ARTIFACT` 会先明确报错，避免被 Rust bootstrap/cargo 环境问题遮蔽 |

### 验证

```bash
python3 scripts/test_install_server_unit.py
python3 scripts/test_smoke_release_gate_unit.py
bash -n scripts/install-server.sh scripts/smoke-release-gate.sh
python3 scripts/test_package_release_unit.py && python3 scripts/test_scripts_quick_unit_coverage.py && python3 scripts/test_smoke_release_gate_unit.py && python3 scripts/test_smoke_provider_federation_unit.py && python3 scripts/test_smoke_web_dual_browser_unit.py && python3 scripts/test_smoke_resident_mainline_unit.py && python3 scripts/test_smoke_cli_channel_unit.py && python3 scripts/test_smoke_auth_registration_unit.py && python3 scripts/test_smoke_shell_dual_http_unit.py && python3 scripts/test_smoke_shell_direct_http_unit.py && python3 scripts/test_smoke_web_shell_unit.py && python3 scripts/test_install_server_unit.py && python3 scripts/test_preview_server_unit.py && python3 scripts/test_start_terminal_shell_unit.py && python3 scripts/test_audit_web_assets_unit.py && python3 scripts/test_lobster_device_id_unit.py && python3 scripts/test_start_web_preview_unit.py && python3 scripts/test_restart_gateway_unit.py && python3 scripts/test_preflight_unit.py && python3 scripts/test_smoke_public_ingress_unit.py && python3 scripts/test_smoke_install_layout_unit.py && python3 scripts/test_start_terminal_unit.py && python3 scripts/test_makefile_unit.py
bash -n scripts/package-release.sh scripts/smoke-provider-federation.sh scripts/smoke-release-gate.sh scripts/smoke-resident-mainline.sh scripts/smoke-cli-channel.sh scripts/smoke-auth-registration.sh scripts/smoke-shell-dual-http.sh scripts/smoke-shell-direct-http.sh scripts/install-server.sh scripts/smoke-web-shell.sh scripts/start-terminal.sh scripts/audit-web-assets.sh scripts/lobster-device-id.sh scripts/restart-gateway.sh scripts/preflight.sh scripts/smoke-public-ingress.sh scripts/smoke-install-layout.sh
zsh -n scripts/start-web-preview.sh
node --check scripts/preview-server.mjs
node --check scripts/smoke-web-dual-browser.mjs
git diff --check
```
