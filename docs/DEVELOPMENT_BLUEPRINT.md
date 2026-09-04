# lobster-chat 开发蓝图 (2026-06-04)

> Codex / Claude Code 每次启动必须读取本文档。
> 记录架构决策、权限模型、UI 规范、待办事项。

---

## 一、系统角色与权限

```
平台管理员（world_steward）
  ├─ 管理世界入口/世界广场场景
  ├─ 管理城主设备白名单
  └─ 城邦违规 → 封禁城主设备 → 城邦下线

城主（city steward）
  ├─ 管理主城/广场场景和热点层
  ├─ 管理居民、审核消息
  └─ 签发邀请码

居民（房主）
  ├─ 管理自己私宅的场景和热点层
  ├─ 私宅背景可更换（白天+夜晚两张图，缺一不可）
  └─ 决定自己私宅的主客访问策略

注册居民（访问者）
  ├─ 登录后才可请求访问他人私宅
  └─ 是否可进入由房主访问策略 + 关系权限决定

未登录访客
  ├─ 不可访问任何居民私宅
  └─ 只能看到登录引导或允许匿名只读的公共入口
```

### 私宅主客访问确权（2026-06-26）

私宅不是默认公开空间。进入他人私宅必须同时满足：

1. 访问者必须是已注册并已登录的 IM 居民，不能用 `访客`、空身份或匿名 viewer 进入。
2. Gateway 必须读取房主保存的私宅访问策略，H5 不能仅凭 `room.id`、`home:<resident>` 或前端状态自行放行。
3. 房主至少可以在两档策略中选择：
   - `registered_all`: 所有已登录注册居民都可以访问。
   - `friends_only`: 只有好友/互相关系居民可以访问。
4. 未配置策略时采用保守默认：不按“所有注册用户可访问”放行；MVP 阶段默认 `friends_only`，只有 Gateway 已确认的好友关系可以进入。
5. 私宅场景展示与私聊消息流必须分层。即使访问者被允许进入私宅场景，也不能因此读取未经授权的历史私聊消息。
6. 未来可扩展 `allowlist`、`blocklist`、一次性邀请等策略，但不能绕过上面的登录与房主确权前提。

2026-06-26 实现状态：
- Gateway 已持久化 `registered_all` / `friends_only` 两档策略，默认 `friends_only`。
- `POST /v1/personal-room/access-policy` 要求 Bearer token 与房主 `resident_id` 匹配。
- 个人房间 shell state 已暴露 `personal_room_access_policy`，用于 H5 显示当前策略。
- H5 住宅页已接入房主专属 `好友` / `注册` 分段控件；控件只在“自己的私宅”显示，提交复用现有 Bearer session。
- Gateway 已新增 `request` / `accept` 两步好友关系流：pending 不解锁，accepted friends 才能访问 `friends_only` 私宅场景。
- `GET /v1/residents?resident_id=<viewer>` 已按访问者投影 `relationship_state` / `relationship_requested_by`，供 H5 判断申请、接受和好友态。
- H5 居民目录和住宅侧栏已接入 `申请好友` / `已申请` / `接受好友` / `好友` 关系入口，提交复用 Bearer session 并通过 Gateway 端点落库。
- H5 点击未授权私宅时会按登录/申请/等待/接受状态给出下一步提示，不再切到当前 shell state 不可见的 room。
- `registered_all` 只开放私宅场景可见性；访客 shell 投影不携带房主私宅历史消息。
- 后续继续做真实移动端验收、关系按钮触控 polish 和更完整的空态视觉，不允许在 H5 本地伪造好友状态。

## 二、热点层权限模型

| 场景 | 编辑权限 | 查看权限 |
|------|---------|---------|
| 私宅（自己的） | 房主本人 | 房主本人 |
| 私宅（别人的） | 房主本人 | 已登录注册居民 + 房主访问策略授权 |
| 主城/广场 | 城主 | 所有人 |
| 世界入口/世界广场 | 平台管理员（我） | 所有人 |

**实现位置**: `conversation_runtime.rs` → `check_scene_edit_permission()`
- 私宅: 检查 `actor_id ∈ conversation.participants`
- 私宅访问: 必须新增/维护独立的 access policy 校验，不能把 1 人 Direct 默认视为公开可见
- 世界入口/广场 (`room:world:entry`, `room:world:square`): `actor_is_world_steward()`
- 公共房间: `actor_is_world_steward()`

## 三、注册与认证

### 用户注册流程
- **邮箱 = 注册ID**（不需要单独的用户名）
- 表单：邮箱地址（即注册ID）+ 用户昵称 → 获取邮箱验证码 → 完成登录
- **手机验证暂不开通**，字段保留 hidden，标记"规划中"
- **注册弹窗必须出现在所有页面**（user.html 跳转页除外）
- 生产邮件验证码由 Gateway 调用通用 HTTPS Webhook 投递；challenge 在运行时锁内创建并持久化，外部邮件调用在锁外执行，未配置或投递失败时重新加锁精确回滚 challenge 并 fail-closed
- 后台居民管理使用 `/v1/admin/residents` 注册投影，可审计未入城账号的脱敏邮箱、状态、注册/验证/最近登录时间

### 设备管控（不是用户注册）
- 城主设备 MAC 白名单：平台管理员通过 `/v1/admin/devices/*` 管理
- 封禁城主设备 → 城邦整个下线
- 设备端点：`GET/POST /v1/admin/devices`, `/add`, `/remove`, `/block`, `/unblock`
- 实现文件：`http_device_routes.rs`

## 四、场景素材管理

### 背景图
- `SceneImageLayer` 新增 `day_image_url` + `night_image_url`（chat-core 合同）
- **白天和夜晚必须成对提供或同时为空**
- 校验在 `validate_scene_image_layer()` 中

### 热点编辑器
- ✅ 已实现在 admin-ds → 房间详情 → 场景配置
- 支持添加/编辑/删除热点（ID、标签、交互提示、X/Y/W/H 坐标）
- 调用 `POST /v1/admin/scene`（带权限校验）
- 场景层更新采用三态语义：字段省略表示保持不变，字段为 `null` 表示清除该层，传对象表示替换；因此选择“默认（无自定义）”或删除全部热点可以真正回到 Gateway 默认投影。

### 预设
- creative-room, main-city, contract-private-room, contract-square-night

## 五、UI 交互规范（不可擅改）

### 场景点击
- 聊天显示时 → 热点层 `pointer-events: none`（不拦截点击，完全透明）
- 点击空白区域 → 隐藏对话框 + 显示热点标签（scene-clear-mode）
- 再次点击 / Esc → 对话框恢复 + 热点隐藏
- **侧边栏和顶栏在 clear mode 下始终保留**（舞台不撑满全屏）

### 已移除的滤镜
- ✅ `body::after` 金色光晕 `rgba(255,210,120,0.10)`
- ✅ `.map-sprite` `mix-blend-mode: multiply`
- ✅ `.map-sprite` `filter: saturate(1.05) contrast(1.06)`
- 保留：暗角 vignette（中性黑色）

### 输入框
- Hub 主城 textarea: `color: #e8ded2`（深色底浅色字,2026-07-30 深色化后）
- 移动端输入字号 ≥16px(防 iOS 聚焦自动放大)

### 三页 chrome 统一(2026-07-31 收口)
- **框架宽度**:住宅/主城/世界广场一律 `100vw` 满宽;历史限宽规则(主城 1440、世界 1640)已清除
- **HUD**:三页统一 sfc 居中 kicker+标题、右侧深色状态丸;世界广场原有左对齐大标题体系已废弃
- **边框**:全部 `#3a2f28` 深色 1px,无金环;金色仅小字符高亮(2026-06-18"三页 HUD 暖金"决定已被本收口取代)
- **导航**:桌面左 rail 深色卡片项、active 冷青左条;移动端(≤820px)三页一律隐藏左栏,由底部 Tab Bar(住宅/主城/世界/我的)承担导航
- **消息气泡**:dark-on-dark `rgba(22,16,12,.88)` + `#3a2f28` 边框,自己=冷青描边;编辑/撤回等动作不常驻气泡,长按/右键呼出底部动作面板(`shell-message-action-sheet.js`)
- **微信受限场景**:`100dvh`、`overscroll-behavior: none`(防下拉误刷新)、safe-area 垫高、composer 抬升至 Tab Bar 之上
- **缓存纪律**:改 CSS/JS 必须同步升版 HTML 引用 `?v=`(CF 边缘强制 max-age=14400 覆盖源站 no-cache,不升版客户端拿不到新样式)

## 六、存储

- 路径：`.lobster-chat-dev/gateway/`
- `device-state.json` — 设备白名单
- `moderation-state.json` — 审核状态
- `timelines/*.postcard` — 消息（二进制 postcard 格式）

## 七、质量基��

- Rust: 512 tests, 0 fail
- JS static: 50 tests, 0 fail
- 零 clippy 警告，零编译警告
- CSS tokens 在 `styles.tokens.css`，所有页面引用

## 八、关键技术债（暂不碰）

- CSS 深度拆分（admin 专属规则散布全文件）
- app.js 场景函数进一步提取
- 系统日志模块对接审计事件

## 九、当前待办

| 优先级 | 事项 |
|--------|------|
| P0 ✅ | world-square + admin-ds 注册弹窗 JS 接线（已接入 `shell-auth-standalone.js`） |
| P0 ✅ | 前端消息发送确认（`?gateway=` 双浏览器 smoke 已验证发送/编辑/撤回/失败重发） |
| P1 ✅ | admin-ds 场景编辑器加 day/night URL 输入（复用 Gateway `SceneImageLayer` 成对校验） |
| P1 ✅ | 场景编辑器统一 16:9 逻辑画布：背景、热点、拖拽/缩放共用同一坐标系；`aspect_ratio_permyriad` 明确为高度/宽度 × 10000，16:9 规范值 5625 |
| P2 ✅ | 城主后台设备管理 UI（已接入 admin-ds 主内容区，复用 `/v1/admin/devices/*`） |
| P2 ✅ | 私宅关系按钮移动端真实验收 + 未授权空态视觉 polish（住宅页状态节点 + 34px 移动按钮 realness） |
| 阻塞 | 多城邦联邦、MLS 加密（PRODUCT_CHARTER 延后） |

## 十、关键文件速查

| 文件 | 内容 |
|------|------|
| `crates/chat-core/src/lib.rs` | SceneImageLayer / SceneHotspotLayer 合同 |
| `conversation_runtime.rs` | scene 权限校验、验证逻辑 |
| `core_runtime.rs` | 设备管理、审核持久化 |
| `auth_runtime.rs` | 注册认证、设备黑白名单 |
| `http_device_routes.rs` | 设备管理 API（5 端点） |
| `shell-scene-runtime.js` | 场景交互（click / clear-mode / labels） |
| `admin-ds.js` | 管理后台（场景配置、热点编辑器） |
| `shell-auth.js` | 前端认证流程（initAuth） |
| `scripts/lobster-device-id.sh` | 本地 MAC 探针 |

## 美术风格强约束（2026-06-14 收口）

> 单一信息源：`apps/lobster-web-shell/assets/pixel/ASSET_HANDOFF.md` §"美术风格强约束"。任何 AI / 协作者改场景资产、UI chrome 配色、昼夜机制、热点层之前**必须**先读那一节，禁止凭直觉调暖色。

要点速查：
1. 像素风夜城都市美学；day/night 仅切光线，不改构图与几何骨架。
2. day 资产**严禁蜡黄/奶油/黄褐**渲染。日景应是冷调自然光（蓝天/晨光/阴天），室内地板保持深棕红原木色。
3. UI chrome（HUD / rail / composer / chat-frame）一律 dark-on-dark，不准 cream/金色大面积填色或 `radial-gradient` 暖色罩层。`--scene-gold` 仅作小字符高亮。
4. 昼夜切换：`body[data-time-of-day]` + 三个 html 内联脚本 + pixel-map.css/world-entry.css/world-square.css 切 PNG + app.js 切运行时 URL，**不要**用 `mix-blend-mode: screen` 或半透明暖叠加伪造日光。
5. 热点 `.scene-hotspot`：缩到原区域 1/4，所有状态边框 / 底色 / 阴影 / outline 全透明。详见 pixel-map.css / world-entry.css / world-square.css 末尾的"2026-06-14 热点层透明化"段。
6. 测试锁定：`apps/lobster-web-shell/test/shell-pages-static.test.mjs` 钉住了 day 资产路径与"禁 mix-blend-mode screen"。改资产或 CSS 后先跑这个用例。

## 十一、开发进度日志（2026-07-07）

### 本轮提交（6 commit，分支 `refactor/web-shell-module-extraction`）

| commit | 类型 | 内容 |
|--------|------|------|
| `298758e` | feat(im) | 提交 6-26~28 未提交 WIP：私宅主客访问确权+好友关系流+注册登录+admin-ds 护栏+P3 下沉（35 文件 +3628/-284） |
| `22d60b9` | refactor | userDetailCard 投影 6 函数 → `shell-user-detail-card.js`（*ForState 注入式，+15 单测，app.js -65 行） |
| `ef93132` | refactor | conversationCallout 文案 4 函数 → `shell-conversation-callout.js`（+10 单测，app.js -53 行） |
| `8ae9642` | docs | README 进度更新到 7-07 + CHANGELOG 创建 + CI 覆盖三端确认 |
| — | docs | 端到端双浏览器 smoke 验证记录 |
| `3db3793` | refactor | 消息动作 payload → `shell-message-action-payload.js`（+5 单测） |

### 验证基线（全绿零警告）

- Gateway: **310 tests / 0 fail / 0 warning**
- Web Shell: **1391 tests / 0 fail**（unit + layout + realness）
- TUI: **233 tests / 0 fail**；启动已接入 Gateway `conversation_shell`/`scene_render` hydration，edit/recall 请求符合 `room_id`/`actor` 合同
- 端到端 smoke: `SKIP_BUILD=1 node scripts/smoke-web-dual-browser.mjs` 通过（真实 gateway + 双浏览器消息发送/编辑/撤回/失败重发闭环，503 注入重发测试）
- app.js: **9342 → 8422 行**；继续按职责和实例边界拆分，不以机械行数为目标

## 十二、生产上线与 UI refresh(2026-07-28 ~ 07-31)

### 生产上线(2026-07-28)

- 公网入口 **https://chat.ajw.cn**(Cloudflare Tunnel → AWS EC2 北京 nginx:80 → gateway 127.0.0.1:8787);`lobster-mailer`(Resend 适配,:8791)同机 systemd
- 真实邮箱注册链路、双端 IM 验收 13/13(收发/失败重发/搜索/编辑/撤回/重启恢复)
- 生产实测修复:pretext vendor 化(artifact 不含 node_modules 致模块图全灭)、hub 页 https 回退同源 gateway、loadGatewayState 携带 Bearer(轮询静默 401)、nginx 对 js/css/html 发 no-cache

### UI refresh P0/P1/P2 + 三页统一(2026-07-30 ~ 07-31)

- P0:移动端顶栏单行、气泡/composer 深色化、消息动作收敛长按面板(新模块 `shell-message-action-sheet.js`)
- P1:三页底部 Tab Bar、微信受限场景适配(dvh/overscroll/safe-area/≥16px)
- P2:桌面深色左栏、限宽消息区、退出键中性化
- 三页统一(07-31):框架一律 100vw、HUD 统一 sfc 居中标题、边框统一 `#3a2f28`、移动端三页均无左栏
- 关键坑:pixel-map.css 内 "final contract" `!important` 规则钉死 cream,外部覆盖无效必须直接改写;creative.html 不加载 styles.scene.css,共用规则须放 pixel-map.css;世界页 stage 显式 `grid-column:2` 在单列移动布局塌缩 2px
- 事故(07-31 已修复):macOS bsdtar 手动打包带 xattr 致 65 张场景图解出 0 字节;手动打包必须 `COPYFILE_DISABLE=1`

### 当前验证基线(2026-07-31)

- Web Shell: **1404 tests / 0 fail**(unit + layout + realness);Gateway 312;完整 release gate 通过
- 缓存纪律:HTML 引用统一 `?v=20260731-ui-refresh-r4`,改样式必须升版

### 邮件链路收口 + 双居民生产验收(2026-08-01)

- **备份/恢复演练**:按 DEPLOYMENT §8 stop-tar-start(秒级停机),备份落 `/srv/backups/`;临时目录恢复 9 文件 sha256 与现网全一致;回滚锚点 `bak-7b0218d` 在位
- **Resend 域名 Verified**:chat.ajw.cn(Tokyo ap-northeast-1)经 Cloudflare Auto configure 写入 DKIM/SPF/MX 并 Verified;正式发件人切 `Lobster Chat <no-reply@chat.ajw.cn>`(`LOBSTER_MAILER_FROM` + `LOBSTER_EMAIL_OTP_FROM` 对齐),gmail/qq 真实 OTP 投递均成功;DMARC 可选未配
- **隐藏缺口修复**:管理能力(ban:resident 等)由 `LOBSTER_SUPER_ADMINS` env 或权限组授予,生产未配置致无人可执行;已加 `LOBSTER_SUPER_ADMINS="rsaga"` 与 governance `world_stewards` 对齐,重启 gateway 生效
- **admin 恢复演练**:按 ACCOUNT_APPEAL_RUNBOOK 对脱敏测试居民执行只读调查→ban→unban→audit 留痕(`admin:ban_resident`/`unban_resident`)全通过
- **双居民验收 8/8**:第二测试邮箱真实 OTP 注册;脱敏测试居民之间的双向收发/对端可见/编辑/撤回全走生产 API（邮箱与居民标识不写入公开仓库）
- **运维教训**:OTP challenge 10 分钟过期且 verify 有尝试次数限制,生产验收需用户即时配合收码;mailer 仅记录失败日志,无 fail 即投递成功

### P1 空间交互 polish 收口 + 生产部署(2026-08-02)

- **场景编辑器 UX**:admin-ds 场景模块新增「打开可视化编辑器」入口(scene-editor.html 此前无入口);编辑器 resize 手柄 `::before` 扩触控热区 10→24px、删除钮 18→28px、方向键 0.5%/Shift 2.5% 微调(600ms 合并 undo)
- **移动端触控目标**:口径对齐既有 realness 约定 **≥34px**(不引入 44px 新标准);`.message-action` 26→34、符号 tab 28→34、mention chip 30→34、世界页 HUD 动作 28→34、登录关闭钮 32→36,全部 ≤820px 媒体查询内
- **私宅空态卡片**:新模块 `shell-private-room-locked-card.js`,未授权私宅五态提示从状态条一行字升级为 timeline 区居中卡片(copy 一律用 Gateway 投影原文);**新 shell 模块必须登记 `test/fake-dom.mjs`,否则 67 用例级联失败**
- **cqh 高度基准修复**:`.creative-stage` 加 `container-type: size`,热点层宽度从 `100vh×16/9` 改 `100cqh×aspect`,热点坐标盒与 contain 背景渲染盒严格对齐;`verify-scene-layout` 新增移动竖屏/桌面 2 个 hotspot canvas case(断言用 client 尺寸,cq 单位相对 content-box)
- **admin-ds 热点编辑器去重**:两套表单合并为 `createHotspotListEditor` 共享助手,净减约 60 行,三态清除语义不变
- **生产部署(web 静态)**:复刻 `package-release.sh` 排除规则本地打包(56M,`COPYFILE_DISABLE=1`,sha256 双端校验),备份后 `web-new`+`mv` 原子交换;版本号 `20260802-scene-canvas-cqh`/`touch-targets`/`locked-card`/`scene-editor-entry`/`hotspot-dedup`;新 `?v=` URL 使 CF 旧缓存自然失效无需 Purge;AWS 北京 SSH 大传输后易瞬断,长操作一律 nohup+日志
- **备份脚本化**:`scripts/backup-state.sh`(三重校验+trap 拉起+KEEP=14 轮转)+ systemd timer 每日 03:30 UTC,手动跑通 <1s;DEPLOYMENT §8 已同步
- **完整 release gate 复验**:Rust/TUI/CLI/Web 1412/双浏览器/provider federation/panic scan 全绿

### 当前验证基线(2026-08-02)

- Web Shell: **1412 tests / 0 fail**(unit + layout + realness);完整 release gate 退出码 0
- 生产与仓库代码完全对齐(全部批次已上线);缓存纪律:改动 CSS/JS 必须同步升版 `?v=`

### H5 Gateway shell state fail-closed（2026-08-13）

- **根因**：配置 Gateway 的 H5 在 shell state HTTP/网络失败时可能继续展示生成态或 IndexedDB 旧投影；provider 请求成功时连接 badge 还可能掩盖 IM shell 失败。
- **实现**：Gateway 配置下 shell state 读取失败、空/畸形载荷统一清空房间/消息投影、活动房间和 state version，禁止回退到 sample/cache；有效 Gateway 载荷恢复后重新标记 shell 可用。
- **连接状态**：新增独立 `gatewayShellStateAvailable`，provider 可达不再单独宣称 IM online。
- **预览边界**：无显式 Gateway 的 `file:` 生成预览保持原有 fixture 行为；bootstrap 内 loopback 开发地址不被误判为正式 Gateway。
- **防回归**：Web **1413/1413**；完整 `scripts/smoke-release-gate.sh` 通过，含 Gateway **323**、真实双浏览器、HTTP 双端和 provider federation smoke。
- **部署边界**：仅本地代码、测试和文档已验证，未 SSH、未部署生产。

### H5 Gateway 会话失效闭环（2026-08-13）

- **根因**：H5 `GET /v1/shell/state` 收到 401/403 时原先只清空 shell 投影，没有清除本地 Bearer session；初始化状态更新还可能把“登录已失效”覆盖成“空闲”，SSE/轮询因此持续携带过期会话。
- **实现**：shell state、POST 和导出共用 Gateway 鉴权失败边界；shell state 401/403 现在清除 token、记录失效状态并保留重新登录提示；新 OTP/新 session token 会清除失效标记。
- **防回归**：Web **1418/1418**、layout、realness；新增 shell state 401 运行时回归和 auth controller 失效状态回归。
- **部署边界**：仅本地代码、测试和文档已验证，未 SSH、未部署生产。

### H5 会话失效真实浏览器回归（2026-08-13）

- **覆盖缺口**：H5 `/v1/shell/state` 401 先前已有 controller/runtime 单元回归，但真实双浏览器 smoke 只证明正常双居民消息链路，没有证明真实页面加载后的过期 session 恢复。
- **实现**：双浏览器 smoke 增加独立 H5 browser context，页面初始化写入脱敏过期 session，并延迟第一次 `/v1/shell/state` 401；不触碰正常 index/creative 双居民 context。
- **验收断言**：真实 H5 页面确认 token 清空、身份降为“访客”、状态保留“登录已失效，请重新登录”，并进入登录 surface；当前 H5 合同允许失效后直接打开登录 overlay，admin-ds 仍单独验证 HUD 回到“登录”并可点击打开 overlay。
- **防回归**：Web **1419/1419**、layout、realness；完整 release gate 退出码 0，含 Gateway 323、TUI/CLI、双 HTTP、真实双浏览器（H5/admin-ds 401）和 provider federation smoke。
- **部署边界**：仅本地浏览器 smoke、代码和文档已验证，未 SSH、未部署生产。

### 公网生产状态只读复核（2026-08-13）

- **可达性**：`https://chat.ajw.cn/health` 返回 HTTP 200；`/v1/provider` 返回 `reachable=true`，但当前仍是 `local-memory` / `Disconnected`，不等于已接入真实 provider。
- **鉴权**：未带会话访问 `/v1/admin/summary` 返回 HTTP 401；匿名 `/v1/shell/state` 与 `/v1/shell/events?wait_ms=0` 返回 200，但旧部署仍暴露 `dm:` 私聊投影，不能视为隐私边界已收口。
- **版本证据**：公网 `/v1/version` 返回 404；`/release-manifest.json` 返回 HTML SPA fallback 而非 manifest；`creative.html` 与 `admin-ds.html` 的静态资源最后修改时间仍为 2026-08-02。
- **真实进度修正**：本次 release 输入 `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d` 已包含 H5/admin-ds 过期会话闭环和真实浏览器回归，但当前不能声称这些修复已在公网生效；生产部署仍需单独授权和验收。

### 公网版本追溯 smoke 收口（2026-08-13）

- **旧缺口**：`smoke-public-ingress.sh` 之前只检查页面、health、provider 和无 Bearer 401，无法发现公网缺少 `/v1/version` 或把 `release-manifest.json` 返回成 SPA HTML。
- **实现**：公网 smoke 现在校验运行时与 manifest 的 40 字符 `git_sha`、manifest `application/json` Content-Type 和两者一致性；`EXPECT_RELEASE_GIT_SHA` 可按发布 commit 精确锁定；`production-readiness.sh CHECK_PUBLIC=1` 同步执行版本追溯检查。
- **验证**：本地真实 HTTP fixture 全链路通过；对 `https://chat.ajw.cn` 的只读 smoke 在 `/v1/version` 404 处失败，证明现网旧部署会被新合同拦截；生产未变更。

### Linux 发布制品可审计收口（2026-08-13）

- **发布基线**：GitHub `main` 的可发布代码 commit 为 `d0e80f54539241381f10f3e256ec065b52083f77`；release workflow run `31632458159` 的 verify、x86_64 和 aarch64 jobs 全部通过。
- **制品覆盖**：已生成 `x86_64-unknown-linux-gnu` 与 `aarch64-unknown-linux-gnu` 两种架构的 source、web shell 和 Gateway 发布包；制品保存在外盘验收目录，尚未部署生产。
- **校验合同**：修复 release workflow，使 `SHA256SUMS` 写入相对 tarball 路径；两种架构均在下载目录直接执行 `sha256sum -c SHA256SUMS` 通过，manifest 的 `git_sha` 与发布 commit 完全一致。
- **内容边界**：`file` 已确认 Gateway ELF 架构正确；source/web archive 已扫描 `.env`、私钥/密钥文件、开发临时目录和高置信度 secret literal，未发现脱敏边界违规。
- **后续边界**：公网版本追溯仍需目标主机部署后再验收；生产 SSH、重启、切换和 DNS/TLS 均未执行。P5 native Waku/标准 MLS 仍需用户单独批准方案调研后再进入选型。

### 生产切换前最后一公里复核（2026-08-13）

- **公网现状复核**：只读请求确认 health 200，但 `/v1/version` 仍为 404，`/release-manifest.json` 仍被 SPA fallback 以 `text/html` 返回；未带 Bearer 的 `/v1/shell/state` 还返回旧版私聊投影，当前线上没有新版本追溯证据。
- **安装演练**：`scripts/smoke-install-layout.sh` 已在隔离临时目录中完成预构建 Gateway/Web artifact 安装流程，生成并检查双份 release manifest、systemd unit、Nginx site，以及 health/version/provider/manifest 探针；没有写入 `/opt`、`/var/lib`、`/etc` 或真实服务。
- **门禁结果**：install-server、install-layout、production-readiness 定向单测与脚本语法检查通过；GitHub 主线 CI 和 Linux release CI 仍保持全绿。
- **执行边界**：下一阶段只剩目标 Linux 主机的备份→安装→服务/公网追溯→真实邮箱 OTP→双居民 IM→回滚演练，必须在用户明确授权后执行；P5 native Waku/标准 MLS 仍是独立授权事项。

### 匿名 shell 投影隐私收口（2026-08-13）

- **现网发现**：只读访问 `https://chat.ajw.cn/v1/shell/state` 与 `https://chat.ajw.cn/v1/shell/events?wait_ms=0`（无 Bearer、无 `resident_id`）均返回 200，响应含 `dm:` 会话 ID 和私聊投影；`/v1/admin/summary` 仍为 401。该现象证明旧公网版本的匿名 shell 投影边界不能作为已修复证据。
- **根因**：Gateway `shell_visible_conversations_for_viewer(None)` 原先将非个人 Direct 会话当成匿名可见；个人房间已有过滤，但普通私聊没有同等 fail-closed 处理。
- **修复**：`41eb6589f7ba37060a0823fd3ea6721dc9882a87` 让匿名投影只保留公共房间；居民 scoped 请求继续要求匹配 Bearer。公网 smoke 新增匿名 `/v1/shell/state` 与 `/v1/shell/events?wait_ms=0` 响应不得出现 `"id":"dm:` 的门禁，Gateway 回归覆盖两个无 query 的 HTTP 路径。
- **验证**：Gateway `323/323`、Gateway clippy `-D warnings`、脚本语法/合同通过；release run `31640540602` 的 verify、x86_64 和 aarch64 全绿。两架构制品 SHA256、manifest、ELF 架构、敏感文件名和 source smoke 内容扫描通过。
- **新发布锚点**：已验证可部署制品为 `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d`，制品目录为 `/Volumes/AJW-Data/Projects/lobster-chat-release-6c0dc6a-run31640540602`；该制品之后仅做部署资料更新，尚未 SSH、重启或替换生产服务。

### admin-ds Gateway 会话失效闭环（2026-08-13）

- **根因**：`admin-ds.js` 的 Gateway GET/POST helper 原先只返回 HTTP 失败，没有通知共享认证控制器；过期 Bearer 会让后台继续保留旧身份和登录 HUD。
- **实现**：admin-ds 对 401/403 统一通知 `shell-auth-standalone.js`；经典脚本早于 deferred module 执行时暂存失效状态，认证模块完成 DOM 接线后立即消费；共享 controller 清除 token/challenge、降为访客并恢复登录 HUD。
- **提示优先级**：身份刷新为访客后仍保留“登录已失效，请重新登录”，避免初始化的访客空态覆盖明确的鉴权失败原因；新 OTP/新 token 会清除失效标记。
- **防回归**：定向 auth/admin-ds/H5 回归 130/130；Web **1419/1419**、layout、realness 已通过；完整 release gate 退出码 0，含 Gateway 323、TUI/CLI、双 HTTP、真实双浏览器和 provider federation smoke。
- **部署边界**：仅本地代码、测试和文档已验证，未 SSH、未部署生产。

### admin-ds 会话失效真实浏览器回归（2026-08-13）

- **覆盖缺口**：此前 admin-ds 401/403 只由静态/单元测试覆盖，完整双浏览器 smoke 只打开 index/creative，没有证明 classic `admin-ds.js` 与 deferred standalone auth 在真实页面加载顺序下能闭环。
- **实现**：双浏览器 smoke 新增独立 browser context 的 admin-ds 页面；通过页面初始化脚本写入脱敏 fixture session，并延迟 `/v1/admin/summary` 的 401，让 classic script 先记录 pending failure，再由 deferred auth module 消费。
- **验收断言**：真实浏览器确认 `lobster-session-token` 清空、`lobster-identity` 变为“访客”、状态保留“登录已失效，请重新登录”、HUD 回到“登录”，点击后登录 overlay 正常打开；index/creative 双端消息发送、编辑、撤回和失败重试不受污染。
- **部署边界**：仅本地浏览器 smoke、代码和文档已验证，未 SSH、未部署生产。

### H5 合法空 Gateway 投影保持在线（2026-08-13）

- **根因**：Gateway `ShellState` 对新居民或被可见性过滤的居民可以合法返回空 `rooms` / `conversation_shell.conversations`；H5 之前把“非空会话”误当成 Gateway 可用条件，导致合法空状态被显示为 offline。
- **实现**：新增 Gateway 专用完整投影校验，要求 `state_version` 和至少一个正式数组字段；合法空数组通过并进入空会话态，不触发 sample/cache 回退；不完整、非对象和无版本投影继续清空并 fail-closed。
- **夹具与回归**：fake Gateway 夹具补齐真实 `state_version`；新增 H5 空投影运行时测试、标准化和不完整载荷单测；消息编辑/撤回 SSE 夹具补齐版本游标。
- **防回归**：Web **1416/1416**；完整 `scripts/smoke-release-gate.sh` 通过，含 Gateway **323**、TUI/CLI、双 HTTP、真实双浏览器和 provider federation smoke。
- **部署边界**：仅本地代码、测试和文档已验证，未 SSH、未部署生产。

### P5 mirror source URL 安全合同（2026-08-13）

- **根因**：enabled world mirror 会被 federation read 主动抓取，但原先只做字符串归一化，可能把任意 HTTP 源写入活动配置。
- **实现**：enabled mirror 复用 provider URL 安全合同；disabled mirror 保留配置兼容；持久化 enabled 非法 URL 在启动载入时 fail-closed。
- **验证**：Gateway 323/323，覆盖 enabled/disabled mirror 与 provider URL 合同复用。
- **范围**：这是跨城 mirror 接入前的安全合同，不代表已接入 native Waku 或标准 MLS。

### P5 provider URL 安全合同（2026-08-13）

- **根因**：`/v1/provider/connect` 原先未复用 production readiness 对远端 URL 的 HTTPS 约束，持久化 provider 配置载入也可能绕过该边界。
- **实现**：远端 provider 只接受 HTTPS；dev/test 仅接受 loopback HTTP；URL 校验覆盖 scheme、host、port、userinfo，并在启动载入非法配置时 fail-closed。
- **验证**：Gateway 322/322；现有 provider federation、CLI/env 初始化和生产 readiness 合同保持通过。
- **范围**：这是 native transport 接入前的安全合同，不代表已接入 native Waku，也不代表现有 `crypto-mls` 已成为标准 MLS/E2EE；组件选型须在用户批准开源调研后进行。

### P1 个人房间场景权限收口（2026-08-13）

- **权限根因**：`home:<resident>` 个人房间已由 `personal_room_owner()` 建模，但 shell 场景写入只检查参与者；admin 私宅检查原先只覆盖 `dm:*`，没有兑现“房主编辑”边界。
- **实现**：shell/admin 场景写入现在对个人房间统一要求房主；世界管家既有全局覆盖保持不变；普通双人 `dm:*` 继续允许参与者编辑，避免改变共享私聊既有行为。
- **防回归**：新增 shell 与 admin 的房主/非房主回归测试；Gateway 319/319，完整 release gate（Rust workspace/Clippy、CLI/TUI、Web 1412、双浏览器、provider federation、发布脚本和 panic scan）全绿。
- **部署边界**：仅本地代码、测试和文档已验证，未 SSH、未部署生产；P5 native Waku/标准 MLS 仍须用户批准开源调研后再进入选型。

### 单城 IM 生产切换与真实双居民验收（2026-08-13）

- **发布锚点**：使用 release run `31640540602` 的 `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d`；目标机通过只读预检确认实际为 `x86_64`，安装使用 `x86_64-unknown-linux-gnu` Gateway/Web 制品。
- **备份与部署**：先完成 `/srv/backups/lobster-chat-state-20260813-005851.tar.gz` 一致性备份，再由 `install-server.sh` 校验 manifest/checksum 后安装；Gateway、Mailer、Nginx、Cloudflared 均 active/enabled。
- **配置边界**：目标机原有生产配置保留；因新版本 readiness 要求而缺失的 `LOBSTER_SECURE_SESSION_MASTER_KEY` 以随机值补齐，仅写入 root-only `/etc/lobster-chat/gateway.env`，未写入仓库、日志、Atlas 或回复。
- **公网验收**：`production-readiness.sh CHECK_PUBLIC=1` 和 `smoke-public-ingress.sh` 均通过；公网 `/v1/version` 与 JSON `release-manifest.json` 精确指向发布 SHA，CORS、管理摘要/ logout 401、匿名 shell state/events 不泄露 `dm:` 均通过。
- **真实居民验收**：两个既有居民通过真实邮件 OTP 完成 verify；响应均为 `mailer-webhook` 且无 `dev_code`。已完成双方 auth session、presence、公共房间和私聊收发、编辑、撤回、搜索、认证 SSE、未授权搜索 401、logout 及旧 token 失效验证。
- **持久化验收**：Gateway 重启后两份 Bearer session 仍可恢复，编辑/撤回消息状态保持，版本仍为目标 release；随后注销临时验收 session。
- **后续收口**：生产切换时观察到的 duplicate `server_name _` warning 已在后续 Nginx 站点名硬化中消除；公网路由与发布 SHA 保持不变。
- **结论**：P0 单城集中式 IM 已完成一次可追溯生产切换与 API/SSE 级真实验收；当前不把本轮结果扩展为 native Waku、标准 MLS/E2EE 或跨城完成。

### Nginx 站点名冲突收口（2026-08-13）

- **根因证据**：AWS 北京 RHEL 系 `/etc/nginx/nginx.conf` 自带 `server_name _;`；IM 的 `/etc/nginx/conf.d/lobster-chat.conf` 再次声明 `_`，因此 `nginx -t` 对 IPv4/IPv6 各报一条 conflicting server name warning，非 duplicate default server。
- **实现合同**：`install-server.sh` 新增 `NGINX_SERVER_NAME` 单名称配置与字符/长度校验；默认生成 `server_name "";`，显式生产域名使用带引号值，`listen ... default_server` 行为保持不变。
- **测试合同**：静态合同、隔离 install-layout smoke、显式域名渲染和恶意指令值 fail-closed 均通过；部署文档要求生产安装显式传正式域名。
- **生产验证**：变更前配置备份到 `/srv/backups/lobster-chat-nginx-20260813-0948.conf`；仅修改 IM server block 后 `nginx -t` 无 warning、reload 成功，本机 health/version 与公网 release smoke 通过，运行 Gateway SHA 仍为 `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d`。
- **范围边界**：未修改发行版默认 server block、Gateway 状态、邮件密钥或 Cloudflare 配置；不代表 P5 transport/MLS 已推进。

### S0 源控救援与门禁去抖（2026-08-12）

- **未提交生产 WIP 已收口**：8 月 2 日场景交互、私宅空态、admin-ds 热点去重和备份 timer 已按功能形成原子提交，不再只存在于工作树/生产目录
- **测试生成态隔离**：TUI 新增 `LOBSTER_WEB_GENERATED_DIR` 覆盖项；terminal 与居民主链 smoke 写入各自临时目录，完整门禁后 tracked `generated/bootstrap.json`、`generated/state.json` hash 保持不变
- **provider federation 去抖**：下游 Gateway 改为等待上游 health 后启动，失败时输出下游日志；单项连续 5/5、完整 release gate 均通过
- **备份脚本回归测试**：新增真实 tar/目录清单单测，并修复 `pipefail + grep -q` 可能触发 SIGPIPE 假失败的问题
- **公开仓库脱敏**：生产验收文档不再记录真实测试邮箱和居民标识；未写入 Token、验证码、API key 或凭据值
- **本地验证基线**：Rust workspace、clippy、CLI、认证、居民主链、Web 1412、双浏览器、TUI、provider federation、备份单测和 panic scan 全绿；本阶段未重新部署生产

### 可选加固：empty-note 视觉统一 + DMARC/SPF（2026-09-04，均已完成）

- **empty-note 统一（已完成）**：`.empty-note` 唯一真源从 `styles.chat.css` 移到所有 app.js 页面（index/admin/creative/unified）共同加载的 `styles.creative.css`；规则为 `.empty-note:not(.timeline-empty-card)` 深色小卡片（`rgba(22,16,12,.55)` 底 + `#3a2f28` 边框 + 居中 12px 圆角），修复 creative/unified 页面空态此前完全无样式的问题；时间线空态卡片经 `:not()` 排除，合同不变。
- **缓存纪律**：`styles.chat.css` / `styles.creative.css` 的 `?v=` 统一升为 `20260904-empty-note-unify`（5 页共 7 处引用 + 4 处测试钉住同步）。
- **防回归**：新增 `test/empty-note-unify.test.mjs` 4 用例；Web 单测 **1423/1423**、layout、frontend realness 全绿；headless Chromium 三页探针计算样式与合同一致、0 未捕获错误。未部署生产，随下一发布批次上线。
- **DMARC/SPF（已落地）**：取证修正——Resend 信封域 SPF 一直在正确位置 `send.chat.ajw.cn`（`include:amazonses.com`），DKIM 在位，真正缺的只有 DMARC。经用户授权用最小权限一次性令牌（仅 ajw.cn DNS 编辑，用后即删）添加：`_dmarc.chat.ajw.cn` TXT `v=DMARC1; p=quarantine;` 与 `chat.ajw.cn` TXT `v=spf1 include:send.chat.ajw.cn ~all`（严格对齐辅助）；Google/Cloudflare 双 DoH 复核生效，令牌已删除并验证失效。

### P5.1 恢复：adapter 接线完成、节点构建暂停于 nim 版本矛盾（2026-09-04）

- **第 1-2 步已完成（本地提交 `9aca689`，按暂停纪律未推送）**：`native_rest` 接线进 `lib.rs`（feature-gated），example 改为消费 lib 模块；三档门禁全绿（default 10/10、feature 21/21 含 native_rest 11 个单测、clippy lib+all-targets `-D warnings`、workspace check/clippy/test、fmt）。代码审查确认与 ADR-0001 §4.2/§4.3 一致。
- **第 3 步节点构建暂停**，根因链四条（均非 adapter 代码问题）：cachyos-ai 直连 github 传输层不稳（已启用 ghfast.top 重写，lock sha1 兜底完整性）；`packages_official.json` 曾截断 2290/2918（已补全）；pkgcache 曾被污染（已清空）；**当前阻塞：lock 钉 `nim 2.2.4` 但同钉的 `ffi@53515de` 要求 `nim >= 2.2.6`，nimble 0.22.3 解析矛盾**（上游 CI 同参数理论应过，未复现）。
- **lab 环境已固化**（cachyos-ai `/mnt/gaosu_sata/labs/lobster-chat-p5-waku/`）：固定源码已验证、最小 lobster 工作区、cargo/rustup 工具链、retry 脚本、lab example 构建成功（rsproxy 镜像仅入实验室 CARGO_HOME）。恢复顺序与详细环境见 `docs/ACTIVE_WORK_QUEUE.md` 顶部 2026-09-04 P5.1 区块。

### 真实进度（2026-08-13）

### P5.1 暂停交接（2026-08-13）

- Atlas Proposal `proposal.38c8236eb16492f31a94` 已批准，决策只覆盖隔离的 P5.1 双节点 lab；生产部署、P5.2 标准 MLS 和协议切换不在本次授权内。
- 官方上游核验确认：稳定版 `v0.38.1` 尚无 Messaging REST，实验固定 Logos Delivery master commit `23b0d31e848812ad54f5d5f390854cb8dd26fe89`；生产必须等待包含该 API 的正式发行版，或重新审批固定源码版本。
- 当前本地工作树有默认关闭的 `native-waku-rest` feature、REST adapter、opaque protobuf envelope、原子 `0600` ledger、Store cursor/dedup 恢复和 lab 示例草稿；该代码尚未通过主 agent 审查，`lib.rs` 尚未接线，cargo feature/default/full gate 均未验收。
- release gate 的 feature test/clippy 接线草稿已落盘；相关 Python 合同单测、shell 语法和 `git diff --check` 已通过。这只能证明脚本合同，不代表 Rust adapter 或真实 Waku 链路通过。
- 隔离开发机已把源码、工具链和日志放在外盘；节点构建在 Nimble 读取 GitHub `nim-stew` tags 时因 TLS EOF 退出，未生成节点二进制，未启动 A/B 节点。不得使用 `--noSSLCheck` 规避证书验证。
- 暂停时生产环境完全未动，开发机没有遗留实验节点/构建进程；WIP 未提交、未推送，等待用户下一次明确继续命令。
- 恢复顺序固定为：代码审查与 `lib.rs` 接线 → feature/default/workspace 门禁 → 安全恢复官方节点构建 → loopback-only 双节点 A/B → 双向各 100 条 → adapter/sidecar 重启与 Store 恢复 → 去重/资源/日志脱敏检查 → 文档/Atlas 收口 → 脱敏提交、推送并等待 CI。任一阶段失败都停在该阶段。

| 模块 | 估算 | 说明 |
|------|------:|------|
| P0 单城 IM 闭环 | 100% | 私宅确权、好友流、认证、消息与双浏览器 smoke 已验证；生产主机/域名/邮件链路 2026-08-01 全部复验通过（真实 OTP 双邮箱投递、双居民私聊 8/8） |
| P1 空间交互 | 100% | 16:9 坐标画布 + 场景编辑器 UX/移动端触控/空态卡片/cqh 基准和个人房间场景房主权限已收口；empty-note 视觉统一 2026-09-04 完成（见上方条目） |
| P2 后台运维 | 100% | admin-ds 写操作护栏和注册账号审计投影完成；LOBSTER_SUPER_ADMINS 补齐后 ban/unban 恢复演练与审计留痕 2026-08-01 生产实操通过；备份/恢复演练通过；备份 2026-08-02 起脚本化+每日 timer |
| P3 技术债 | ~95% | app.js 当前 7524 行；admin-ds 热点编辑器 2026-08-02 去重合一；本轮补齐 Gateway shell fail-closed 与合法空投影语义；剩余候选 3/4/5 各 15-28 行且耦合运行时状态，蓝图维持"不做机械搬运"结论 |
| P4 TUI/CLI parity | 100% | CLI/TUI、居民主链、terminal 与 provider federation 已纳入完整 release gate；2026-08-02 完整门禁复验通过 |
| P5 跨城/加密 | 22% | P5.0 Proposal 已批准，P5.1 有未验收 adapter/build WIP；真实双节点 transport、RFC 9420 MLS、H5/TUI 互操作和生产 canary 均未通过 |

### 2026-09-04 暂停收尾：GitHub 同步与 lab Nim 2.2.6 就绪

- GitHub `main` 已同步至 `7a358b8`（P5.1 接线 `9aca689` + 暂停交接文档），CI run `33877122445` 全绿；生产锚点不变（`6c0dc6a`）。
- cachyos-ai 实验室已装 Nim 2.2.6（官方源 nim-lang.org，sha256+`xz -t` 双验证；国内镜像复核无 Nim 发行版，OVERSEAS_FALLBACK 门禁已入档 Atlas）；`scripts-retry-nimble-setup.sh` PATH 已切换至 2.2.6。
- P5.1 节点构建仍暂停：下一动作为在 lab 重跑 nimble setup 验证 lock 解析（lock 钉 nim 2.2.4 vs `ffi@53515de` 要求 >=2.2.6 的矛盾），通过后按 `docs/ACTIVE_WORK_QUEUE.md` 最新区块入口②-⑥推进。

### 2026-09-04 全量复盘与方向重排（工程强 / 运营弱）

复盘结论（三层口径）：MVP 冻结范围 ~97%；完整愿景 ~55%；生产工程成熟度 ~85%。
Rust 55,355 行 + 前端源码 ~37,000 行 + 2,187 测试全绿 + 可追溯生产发布链构成工程底座；
但运营侧资产接近零（唯一触点 README 微信群二维码）、消息仅纯文本 1-2000 字符、
无推送通知、无媒体消息——这三项是"让真实用户留下来"的第一拦路石。

**新优先序（取代"P5 下一主战场"排布；P5 降级为背景节奏）**：

| 序 | 事项 | 理由 | 量级 | 状态 |
|----|------|------|------|------|
| 1 | 图片消息链路（上传端点+对象存储+气泡渲染降级） | 运营第一缺口；M2 消息引擎从 90% 补到 97% | 1-2 周 | 未开始 |
| 2 | WebPush 通知（Gateway 事件订阅 + Push API） | IM 留存底线，无推送勿谈运营 | 1 周 | 未开始 |
| 3 | R3 panic 隔离 | release 移除 panic=abort + 请求边界 catch_unwind 500 兜底 + 锁投毒恢复 | 1-2 天 | ✅ 2026-09-04 完成 |
| 4 | R2 增量写（timeline 改 append-only 帧流） | 消息量增长前修，全量重写 IO 是首个性能悬崖 | 3-5 天 | 未开始 |
| 5 | PWA manifest 最小版 + 加桌引导 | 移动端安装入口 | 1 天 | 未开始 |
| 6 | R1 锁拆分（读路径快照化） | 等 >5 并发用户再做不迟 | 后置 | 未开始 |
| 7 | P5.1 lab 恢复 | 按既有暂停合同走背景节奏，不设 deadline，不顺手改生产 | 按队列 | 暂停中 |

**运营路线（三阶段）**：①自用精品——把"我和狗蛋儿的家"双人场景做到极致
（补媒体+推送后即真正可日常自用）；②小圈子城邦内测——微信群为种子池，
邀请码体系已就绪，前置硬条件=推送+媒体+加桌；③开源社区——CLI 智能体通道
（OpenClaw/Codex/Claude Code 直发消息）为差异化卖点，补英文 Agent 快速开始文档。

**明确不做**：现阶段投入通用 IM 市场竞争或以"去中心化/反平台"叙事拉新。

**架构风险登记（复盘 R1-R6）**：R1 全局粗锁 / R2 全量重写 IO / R3 panic=abort
（已修）/ R4 上帝对象 / R5 加密骨架承载生产 / R6 双后台体系。R3 见上方完成项；
R1/R4 在自用规模下不触发，登记在案待用户规模信号。

### 关键发现：6-26~28 WIP 曾未提交

进入时工作树遗留 6-26~28 三天密集开发的 3628 行 WIP（私宅访问确权+好友关系流+前端纯状态下沉），测试全绿却从未 commit。本轮首要动作是锁定这批 WIP（commit `298758e`），消除最大风险。codex 日记与 CC 转录显示 7 月精力在 WVI 等其他项目，IM 处于停摆状态。

### 下一步候选（按优先级；2026-09-04 复盘重排）

1. **单城集中式 IM 已完成面保持**：P0/P1/P2/P4 全部收口并生产验证；日常按 release gate + 备份 timer 维持
2. **运营缺口补齐主线**：图片消息 → WebPush → PWA（见上方复盘新优先序 1/2/5）
3. **P5 跨城/MLS 加密（背景节奏）**：PRODUCT_CHARTER 后置项；P5.0 已审批，P5.1 接线已推送（`9aca689`，CI 绿），节点构建暂停在 nimble 解析验证之前（lab Nim 2.2.6 已就绪）；按 ACTIVE_WORK_QUEUE 最新暂停区块恢复，不再作为默认主战场
4. **可选加固（不阻塞）**：均已完成——DMARC/SPF TXT（2026-09-04）、empty-note 视觉统一（2026-09-04）、R3 panic 隔离（2026-09-04）。当前无剩余低价值加固项。

（已完成的候选：生产环境复验 2026-08-01；P1 空间交互 polish、备份脚本化/定期化 2026-08-02；DMARC/SPF TXT、empty-note 视觉统一、R3 panic 隔离 2026-09-04。）

剩余 app.js 候选 3/4/5（消息 payload 已做；房间状态/连接/同步文案、shellMode 视图状态文案）收益小（各 15-28 行）且多耦合运行时状态或 DOM，边际递减。

### 下一阶段执行规划（2026-08-13；生产收口优先）

> 执行状态（2026-08-13）：本规划的 0→6 阶段已完成；当前生产状态、备份、真实 OTP/双居民验收和 Atlas 收尾以本页上方“单城 IM 生产切换与真实双居民验收”及 `docs/DEPLOYMENT.md` 为准。下方保留原始执行规划作为可重复 runbook。

下一阶段不再以继续拆 `app.js` 或增加低收益 UI polish 为主，而是把已经通过本地门禁的单城 IM 版本安全地变成**可追溯、可验收、可回滚的生产发布**。每一阶段必须满足退出条件后才能进入下一阶段；任何失败都停在当前阶段，不带故障进入真实居民验收。

#### 0. 固定发布锚点与执行资料

当前已验证的发布锚点固定为：

| 项目 | 值 |
|------|-----|
| `RELEASE_GIT_SHA` | `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d` |
| GitHub release run | `31640540602` |
| 制品目录 | `/Volumes/AJW-Data/Projects/lobster-chat-release-6c0dc6a-run31640540602` |
| release 输入 GitHub `main` | `6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d`；该制品之后仅做部署资料更新 |
| x86_64 target | `x86_64-unknown-linux-gnu` |
| ARM64 target | `aarch64-unknown-linux-gnu` |
| 安装/公网合同 | `docs/DEPLOYMENT.md` §3–§9 |

本次只改文档不会改变该 Gateway/H5 制品的代码内容。生产安装必须以 `release-manifest.json.git_sha` 为准，而不是以当时的本地 HEAD 或服务别名为准；如果重新打包，则先产生新的完整 Git SHA，再重新跑 release gate、制品校验和 manifest 复核，不能混用旧制品与新 SHA。

#### 1. 本地发布前门禁（无生产写入）

**执行人**：Codex/开发机。**入口**：固定发布锚点和目标架构已确认。

1. 根据目标主机 `uname -m` 选择 `x86_64` 或 `aarch64` 制品；在对应目录直接执行 `sha256sum -c SHA256SUMS`。
2. 检查 `release-manifest.json` 的 `git_sha` 等于 `RELEASE_GIT_SHA`，Gateway tarball 经 `file` 确认为对应 ELF 架构；不把 source/web archive 的内容当成 Gateway 已运行证据。
3. 复跑文档和脚本门禁：`git diff --check`、`bash -n scripts/{install-server.sh,backup-state.sh,production-readiness.sh,smoke-public-ingress.sh,smoke-install-layout.sh}`、`scripts/smoke-install-layout.sh`。
4. 仅当发布代码发生变化时，才运行完整 `RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh` 并重新生成制品；文档-only 提交沿用已验证 release run，但不能在报告中声称它重新编译过 Gateway。

**退出条件**：目标架构、三类制品 checksum、manifest SHA、安装 layout 演练全部通过；无任何制品来自 dirty worktree；外盘临时目录之外不复制大文件。

#### 2. 目标 Linux 主机只读预检（需要生产授权）

**执行人**：获得用户明确授权的目标主机操作员。当前 Atlas 事实 `host.aws-ec2-beijing.scheduling.agent_execution_allowed=false`，因此 Codex 不直接 SSH、重启或切换该主机；授权应明确包含“在 aws-beijing 执行 IM 生产切换”。

只读确认以下项目，并只回报状态、路径和版本，不回显环境文件内容、Bearer、OTP、API key 或私钥：

| 检查 | 通过标准 | 失败处理 |
|------|----------|----------|
| 架构/空间/内存 | 与制品 target 匹配；磁盘和备份目录有余量 | 换正确制品或先清理/扩容，不安装 |
| Gateway/mailer/Nginx | unit、监听端口和 Nginx 配置可定位 | 先修服务依赖，不重启切换 |
| 环境文件 | 文件存在、权限正确、只报告变量名；生产 readiness 不拒绝 | 不复制/改写 secret，回到配置维护流程 |
| 当前版本 | 记录当前 `/v1/version`、manifest 类型和服务状态 | 作为回滚锚点，不覆盖旧制品 |
| 备份目录 | `/var/lib/lobster-chat` 与 `/srv/backups` 可写且有空间 | 备份不可验证时禁止安装 |

本阶段只执行 `scripts/preflight.sh`、`systemctl is-active/status`、`nginx -t`、本机 health/version/manifest 的读取检查，不执行 `restart`、`install-server.sh` 或状态写入。

#### 3. 备份、安装与服务恢复

**入口条件**：第 2 阶段只读预检通过，且备份目录可用。

1. 先运行已脚本化备份：`sudo bash /opt/lobster-chat/scripts/backup-state.sh`；确认 tarball 非空、包含 timelines、可解压，且 Gateway 被 trap 拉回 active。
2. 保留当前 Gateway/H5 的版本和回滚制品；对 `/etc/lobster-chat/*.env` 只做受保护的运维备份，不将其复制到仓库、日志或公开工单。
3. 使用与目标架构匹配的 Gateway、Web 和同目录 `release-manifest.json` 调用 `scripts/install-server.sh`；必须传 `RELEASE_MANIFEST`，让安装器在替换前校验 checksum 和 SHA。
4. `daemon-reload` 后按 runbook 恢复 Gateway/mailer，先验证本机 `127.0.0.1:8787/health`、`/v1/version` 和 `release-manifest.json`，再验证 Nginx；不以 systemd `active` 单独作为成功标准。

**停止条件**：备份失败、manifest/checksum 不符、Gateway 本机 health/version 失败、Nginx 配置失败、mailer 不可达，立即停止在本阶段并使用原制品恢复；不得进入公网登录验收。

#### 4. 公网版本追溯与安全边界

本机检查通过后，执行：

```bash
BASE_URL=https://chat.ajw.cn \
EXPECT_RELEASE_GIT_SHA=6c0dc6a5c5429b54f15bc0c7c403e0e393f0488d \
  scripts/smoke-public-ingress.sh
```

并用 `CHECK_PUBLIC=1` 的 `scripts/production-readiness.sh` 做同一轮只读检查。必须同时满足：`/health` 200、`/v1/version` 为 JSON、`/release-manifest.json` 为 JSON 而非 SPA HTML、两处 `git_sha` 完全一致、受保护摘要和 logout 在无 Bearer 时返回 401、正式 CORS origin 正确。`/v1/provider` 显示 `local-memory`/`Disconnected` 只能说明当前 provider 状态，不能替代居民 IM 或外部 provider 的验收。

**停止条件**：任何版本 404、manifest `text/html`、SHA 漂移、鉴权边界变宽、首页/住宅页/后台页不可达，都先回滚或修复代理，再继续；不在旧公网部署上做真实邮箱和居民数据测试。

#### 5. 真实 OTP 与双居民 IM 验收

仅在第 4 阶段通过后，使用两组经用户批准的测试邮箱和脱敏居民标识；报告只写结果，不写邮箱、resident ID、token、challenge ID 或验证码。

| 组别 | 必测项 | 通过标准 |
|------|--------|----------|
| 注册/会话 | OTP 实际到达、verify、shell state、logout、旧 token 失效 | 邮件收件箱真实收到；响应不含 `dev_code`；退出后旧 token 401 |
| 公共房间 | A→B 发送、B 可见、编辑、撤回、搜索 | 双端最终状态一致，搜索遵守参与者权限 |
| 私聊 | direct/open、双向发送、编辑、撤回 | 非参与者不可读；双方状态一致 |
| 失败恢复 | 人为一次网络失败后重发 | 失败气泡可重发，最终 committed copy 不重复 |
| 持久化 | 刷新页面、重启 Gateway 后恢复 | 消息、注册、已读和会话按既有合同恢复 |
| 管理后台 | 居民/房间/审计读取，按 runbook 做一次恢复动作 | 成功/失败反馈真实，`audit-log.json` 有脱敏留痕 |

每个场景记录“请求入口—HTTP/SSE 结果—页面最终态—是否重启后复现”；只保存脱敏摘要。若 OTP 真实投递失败、消息重复/丢失、越权可读、登出后旧 token 仍有效或后台写操作假成功，立即进入回滚判定。

#### 6. 回滚判定与收口

任一阶段触发停止条件时，优先保留故障证据和当前坏状态副本；按 `docs/DEPLOYMENT.md` §8 安装上一版匹配架构制品。只有确认新版本造成状态格式不兼容时才恢复旧状态备份，不能为修复代码问题默认覆盖用户数据。回滚后重复 `/health`、本机 version/manifest、公网 smoke 和最小真实登录验证。

本阶段的完成门槛是：

1. 公网版本和 manifest 精确指向部署制品 SHA；
2. Gateway、mailer、Nginx、TLS/公网入口均有真实可用证据；
3. 未授权 401、CORS、logout 和不泄露 `dev_code` 的安全边界通过；
4. 两组真实 OTP 收件、双居民公共房间/私聊矩阵和失败重发通过；
5. 备份包、安装 manifest、验证摘要和（如发生）回滚结果均可追溯且脱敏；
6. Atlas 写入本次部署的 finding/change/verification/checkpoint，释放 lease 后再 finish task。

#### 7. P5 保持独立授权门

P5 不与本次单城生产切换混合。当前 HTTP gateway federation 与自研 AES-GCM `crypto-mls` 骨架不得包装成 native Waku 或标准 MLS。用户已于 2026-08-13 允许限时开源调研并批准 Atlas Proposal `proposal.38c8236eb16492f31a94`；[`ADR-0001`](adr/0001-p5-native-waku-and-standard-mls.md) 已采纳为 P5.1 隔离实验合同。该批准不覆盖 P5.2、生产基础设施或生产切换；feature-gated lab 未完成真实验收前不得宣称 native Waku 已落地。

### 未入主线的实验产物（保留 untracked）

按 scope 收紧原则（ACTIVE-im [ACT-013]）未接入页面、不入主线提交：
- neon-pixel 主题：`styles.theme-neon-pixel.css` / `docs/UI_DESIGN_THEME_NEON_PIXEL.md` / `docs/mockups/neon-pixel-*.png`
- 私宅素材：`apps/lobster-web-shell/assets/pixel/private-room-alt01-*.png`
- 临时：`.tmp/`、`apps/lobster-web-shell/output/`、`styles.world-square.css.tmp`
