# lobster-chat 部署与生产验收

本文是当前单城集中式 IM 的生产部署真值。跨城 Waku relay、MLS 和链上锚定仍按 `PRODUCT_CHARTER.md` 后置，不阻塞单城上线。

## 1. 当前发布基线（2026-08-13）

- Gateway：323 tests / 0 failed / clippy 0 warning
- Web Shell：1419 tests / 0 failed，layout 与 frontend realness 通过
- TUI：235 tests / 0 failed；Gateway 配置下启动读取 `conversation_shell`/`scene_render`
- CLI：148 tests / 0 failed（119 unit + 24 integration + 5 额外测试）
- 支撑 crates：`crypto-mls` 24、`ai-sidecar` 7、`chat-core` 20、`chat-storage` 18 tests，均 0 failed
- 完整门禁：`RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh`
- 当前可发布安全修复：GitHub `main` `7031f624645302df0e01a5872db7a8a8111a6197`；release run `31637554649` 的 verify、x86_64、aarch64 均通过
- 当前制品目录：`/Volumes/AJW-Data/Projects/lobster-chat-release-7031f62.2tF0mH`；两个 target 的 `SHA256SUMS` 和 manifest 均已独立复核
- H5 主入口：`index.html`（主城群聊）、`creative.html`（住宅/私聊）、`admin-ds.html`（管理后台）

## 2. 支持环境

生产安装脚本面向带 systemd 和 Nginx 的 Linux：

- Ubuntu 22.04+、Debian 12+ 或同类发行版
- x86_64 或 aarch64
- 从源码构建需要 Cargo/Rust 1.85+
- 使用目标架构预编译 artifact 时，目标机不需要 Rust/Node.js
- 最低建议 1 GiB 内存、2 GiB 可用磁盘；状态和备份空间另计

先在目标机执行：

```bash
bash scripts/preflight.sh
```

## 3. 构建与打包

发布前在仓库根目录执行完整门禁：

```bash
RUN_PREFLIGHT=0 INCLUDE_PROVIDER_FEDERATION=1 scripts/smoke-release-gate.sh
```

构建目标架构 artifact：

```bash
scripts/package-release.sh
```

发布脚本默认拒绝 dirty worktree，并在 `dist/release-manifest.json` 写入完整 Git SHA、构建时间、目标平台以及 source/web/gateway 三类产物的 SHA-256。使用预构建产物安装时同时传入 manifest，安装器会在替换文件前核对 checksum：

```bash
sudo GATEWAY_ARTIFACT=/path/lobster-waku-gateway-<target>.tar.gz \
  WEB_ARTIFACT=/path/lobster-web-shell.tar.gz \
  RELEASE_MANIFEST=/path/release-manifest.json \
  bash scripts/install-server.sh
```

只要传入预构建制品，`RELEASE_MANIFEST` 就是必填项。部署后安装器会以 `/v1/version` 核对 Gateway 编译 commit，并通过 `/release-manifest.json` 暴露部署 artifact；运行版本与 manifest 的 `git_sha` 不一致时安装失败。

标准产物：

```text
dist/lobster-chat-source.tar.gz
dist/lobster-web-shell.tar.gz
dist/lobster-waku-gateway-<target-triple>.tar.gz
dist/release-manifest.json
```

在 macOS 开发机无法直接生成 Linux Gateway 时，可手动触发 GitHub Actions 的
`lobster-chat-release` 工作流。工作流先运行 Rust workspace 与 Web 全量测试，再由 x86_64 与 ARM64 Linux runner 按各自 target triple 显式编译并校验 runner 架构，分别上传源码、H5、目标架构 Gateway 和可在任意下载目录直接执行 `sha256sum -c SHA256SUMS` 的相对路径 `SHA256SUMS`。

当前 Linux artifact 目标为：

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`

ARM64 job 使用 GitHub 官方 `ubuntu-24.04-arm` runner；该 runner 为 Public Preview，若仓库组织策略禁用该 runner，必须改用已注册的等价 ARM64 runner，并保持 artifact target triple 不变。

macOS 构建出的 Darwin binary 不能安装到 Linux。安装器会校验 artifact 文件名中的 target triple，但跨架构产物仍应由对应 CI runner 或交叉编译环境生成。

## 4. Linux 安装

推荐使用 Gateway + H5 两个预编译产物：

```bash
sudo \
  GATEWAY_ARTIFACT=./dist/lobster-waku-gateway-x86_64-unknown-linux-gnu.tar.gz \
  WEB_ARTIFACT=./dist/lobster-web-shell.tar.gz \
  ./scripts/install-server.sh
```

默认路径：

| 内容 | 路径 |
| --- | --- |
| 安装根目录 | `/opt/lobster-chat` |
| Gateway | `/opt/lobster-chat/bin/lobster-waku-gateway` |
| H5 | `/opt/lobster-chat/web` |
| 持久化状态 | `/var/lib/lobster-chat` |
| systemd unit | `/etc/systemd/system/lobster-waku-gateway.service` |
| Nginx site | Debian: `/etc/nginx/sites-available/lobster-chat`; RHEL 系: `/etc/nginx/conf.d/lobster-chat.conf` |

Gateway 默认只监听 `127.0.0.1:8787`，公网流量由 Nginx 接入。不要把 Gateway 裸端口直接暴露到公网。

## 5. 生产环境变量

安装脚本不会把密钥写入仓库或 artifact。使用 systemd drop-in 持久化生产配置：

```bash
sudo install -d -m 0750 /etc/lobster-chat
sudo install -m 0600 /dev/null /etc/lobster-chat/gateway.env
sudoedit /etc/lobster-chat/gateway.env
```

`/etc/lobster-chat/gateway.env`：

```dotenv
LOBSTER_CORS_ORIGIN=https://chat.example.com
LOBSTER_DEV_AUTH_BYPASS=0
LOBSTER_DEV_EMAIL_OTP_INLINE=0
LOBSTER_EMAIL_OTP_MAILER_URL=http://127.0.0.1:8791/lobster/email-otp
LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN=replace-with-secret
LOBSTER_EMAIL_OTP_FROM="我和狗蛋儿的家 <no-reply@chat.ajw.cn>"
LOBSTER_SECURE_SESSION_MASTER_KEY=replace-with-at-least-32-character-secret

> 含空格/尖括号的值必须加双引号:gateway.env 会被 systemd 读取,也会被
> scripts/production-readiness.sh 用 bash source,不加引号会导致 bash 语法错误。
```

启用 gateway-to-gateway 上游桥接时，接收端和发起端必须分别配置专用凭据；不要复用居民 session、管理员 token 或邮件 token：

```dotenv
# 上游接收端：校验 /v1/waku 的 Authorization: Bearer
LOBSTER_GATEWAY_FEDERATION_TOKEN=replace-with-dedicated-secret

# 下游发起端：仅随 /v1/waku 请求发送，不写入 provider-config.json
LOBSTER_WAKU_UPSTREAM_TOKEN=replace-with-the-upstream-secret

# TUI 等受信客户端直连 Gateway 协议时使用；应匹配接收端 federation token
LOBSTER_WAKU_GATEWAY_TOKEN=replace-with-the-gateway-secret

```

生产模式下 `/v1/waku` 缺失或携带错误 federation token 一律返回 401；原始 token 只从环境读取，runtime 仅保留哈希，Debug 和持久化状态均不得出现明文。
`secure-sessions.json` 使用该 master key 派生 AES-256-GCM 存储密钥；`POST /v1/direct/open` 只返回会话元数据，不返回 `group_key`。生产 readiness 会拒绝缺失或过短的 master key。当前 MLS 仍是骨架，不应据此宣称标准 MLS/E2EE。

`LOBSTER_EMAIL_OTP_MAILER_URL` 指向同机部署的 `apps/lobster-mailer`（推荐，
loopback HTTP 是 Gateway 唯一放行的非 HTTPS 情形）；使用外部邮件 Webhook
时必须为 HTTPS。

### 5.1 同机部署 lobster-mailer（真实邮件 OTP）

`apps/lobster-mailer` 是无第三方依赖的 Node 22 服务，把 Gateway 的 OTP
webhook 转成 Resend API 调用。前置条件：Resend 账号、API Key、完成发信域名
验证。

```bash
sudo install -d -m 0755 /opt/lobster-chat/mailer
sudo cp -R apps/lobster-mailer/src /opt/lobster-chat/mailer/
sudo install -m 0644 apps/lobster-mailer/deploy/lobster-mailer.service \
  /etc/systemd/system/lobster-mailer.service
sudo install -m 0600 /dev/null /etc/lobster-chat/mailer.env
sudoedit /etc/lobster-chat/mailer.env
```

`/etc/lobster-chat/mailer.env`：

```dotenv
LOBSTER_MAILER_BEARER_TOKEN=<与 gateway.env 中 LOBSTER_EMAIL_OTP_MAILER_BEARER_TOKEN 相同>
RESEND_API_KEY=re_xxxxxxxx
LOBSTER_MAILER_FROM="我和狗蛋儿的家 <no-reply@chat.ajw.cn>"
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now lobster-mailer
curl -fsS http://127.0.0.1:8791/health
```

验收：发送 OTP 后真实邮箱收到验证码邮件；mailer 日志只记录上游状态码，
不输出 Bearer 或验证码正文。

部署前可执行 scripts/production-readiness.sh 做只读配置检查。设置 CHECK_PUBLIC=1 并提供 BASE_URL=https://chat.example.com 后，还会探测公网 health、provider 和 CORS；脚本不会输出 Bearer 密钥。

添加 drop-in：

```bash
sudo install -d -m 0755 /etc/systemd/system/lobster-waku-gateway.service.d
sudo tee /etc/systemd/system/lobster-waku-gateway.service.d/10-production.conf >/dev/null <<'EOF'
[Service]
EnvironmentFile=/etc/lobster-chat/gateway.env
EOF
sudo systemctl daemon-reload
sudo systemctl restart lobster-waku-gateway
```

要求：

- `LOBSTER_CORS_ORIGIN` 必须是单一正式 `https://` H5 origin，不能是 `*`
- 两个 `LOBSTER_DEV_*` 必须为 `0` 或不设置；`scripts/production-readiness.sh` 会拒绝未知值
- 邮件 Webhook 必须使用 HTTPS；只有 loopback 集成测试允许 HTTP
- Bearer 密钥只进入环境文件和请求 header，不进入仓库、URL、日志或状态目录
- Nginx 必须透传 `Authorization` header（默认行为；自定义代理规则不得清除）
- 安装脚本生成的 Nginx 配置会显式设置 `proxy_set_header Authorization $http_authorization`，避免依赖代理默认行为
- H5 Gateway 页面不能把 URL 的 `identity` 参数当作认证凭据；无 Bearer session 时按 `访客` 加载，正式输入与发送必须禁用
- `identity` + `qa=browser`/`qa=manual` 只允许 loopback Gateway 的显式合成 QA 验收，不能用于正式域名或公网身份冒充
- H5 消息搜索 `/v1/shell/messages/search` 必须携带 `resident_id` 和匹配的 Bearer session；无会话返回 `401`，非参与者不得得到私聊正文
- H5 与 admin-ds 已登录态的退出入口调用 `POST /v1/auth/logout`；服务端成功后旧 Bearer 必须立即失效，网络失败只能显示待重试，不能伪报已撤销
- OTP、challenge ID 和 Bearer session token 必须使用操作系统 CSPRNG；随机源不可用时认证请求必须失败关闭

## 6. TLS 与公网入口

`install-server.sh` 只生成 HTTP Nginx 站点。正式域名必须在它前面配置可信 TLS，可使用云负载均衡、CDN、Certbot 或已有反向代理。

从外部网络执行：

```bash
BASE_URL=https://chat.example.com \
EXPECT_RELEASE_GIT_SHA=<40-character-commit-sha> \
  scripts/smoke-public-ingress.sh
```

该脚本检查首页、H5 住宅页 `creative.html`、管理后台 `admin-ds.html`、GET/HEAD `/health`、`/v1/provider`、`/v1/version` 与 JSON `release-manifest.json` 的 SHA 追溯、无 Bearer 访问管理摘要与 logout 的 `401` 边界，以及匿名 `/v1/shell/state` 和 `/v1/shell/events?wait_ms=0` 不暴露 `dm:` 私聊投影；设置 `EXPECT_CORS_ORIGIN` 时还会校验 health 的 CORS 响应。`EXPECT_RELEASE_GIT_SHA` 应填写本次部署制品对应的完整 commit SHA；它不能替代 DNS、证书链、真实邮箱和登录后的浏览器验收。

## 7. 必须完成的生产验收

### 服务与安全边界

```bash
systemctl is-active lobster-waku-gateway
nginx -t
curl -fsS http://127.0.0.1:8787/health
curl -fsS https://chat.example.com/health
curl -fsS https://chat.example.com/v1/provider
```

- 无 Bearer token 读取受保护的 `/v1/admin/summary` 应返回 `401`
- OTP request 响应不得包含 `dev_code`
- 浏览器请求的 `Access-Control-Allow-Origin` 必须是正式 H5 origin，不是 `*`
- `journalctl -u lobster-waku-gateway` 不得输出邮件 Bearer 密钥或 OTP 正文

### 真实邮箱注册链路

1. 使用未注册真实邮箱调用/操作 OTP request。
2. 确认邮件实际到达，不以 Webhook `2xx` 代替收件验收。
3. 使用邮件中的验证码完成 verify。
4. 确认 session token 可读取居民 shell state。
5. logout 后确认旧 token 立即失效。
6. 在 admin-ds 居民详情确认脱敏邮箱、注册/验证/最近登录时间正确。

### H5 双端 IM

- 两个居民分别登录主城/住宅入口
- 公共房间和私聊各完成发送、接收、编辑、撤回
- 登录居民在当前可见公共房间/私聊搜索消息；退出登录或切换为非参与者后，搜索不得返回受限会话正文
- 人为制造一次网络失败，确认失败气泡可重发且最终没有重复 committed copy
- 刷新页面和重启 Gateway 后确认消息、注册、已读和会话仍可恢复

### 运维能力

- admin-ds 能读取居民、房间、审计日志和配置
- 禁用/恢复居民、冻结/解冻房间、消息审核操作有成功或失败反馈
- `audit-log.json` 能追踪高风险动作
- 按 [账号申诉操作手册](ACCOUNT_APPEAL_RUNBOOK.md) 演练一次只读调查和一次恢复动作

## 8. 备份、恢复与回滚

状态目录是单城节点的核心资产。备份前停止 Gateway，保证多个 JSON 文件处于一致时间点。

**推荐：脚本化备份（2026-08-02 起）**

```bash
sudo bash /opt/lobster-chat/scripts/backup-state.sh
```

脚本（源码 `scripts/backup-state.sh`）固化本流程并附加：tarball 非空/含 timelines/可解压三重校验、磁盘水位检查、无论成败都拉起 Gateway（trap）、按 `KEEP=14` 轮转。定期化由 systemd timer 承担：

```bash
sudo install -m 0644 scripts/systemd/lobster-chat-backup.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now lobster-chat-backup.timer   # 每日 03:30 ±15min
systemctl list-timers lobster-chat-backup.timer          # 查看下次触发
sudo systemctl start lobster-chat-backup.service         # 手动触发一次
journalctl -u lobster-chat-backup.service                # 查看备份日志
```

手动流程（脚本不可用时的兜底）：

```bash
sudo systemctl stop lobster-waku-gateway
sudo tar -C /var/lib -czf /srv/backups/lobster-chat-state-$(date +%Y%m%d-%H%M%S).tar.gz lobster-chat
sudo systemctl start lobster-waku-gateway
```

恢复前先保留当前坏状态副本，再解压已验证备份，确保目录权限与原安装一致。恢复后必须重新执行 `/health`、真实登录和双端消息验收。

版本回滚：

1. 部署前保存当前 Gateway/H5 artifact 和状态备份。
2. 重新运行 `install-server.sh` 安装上一版匹配目标架构的 artifact。
3. systemd drop-in 和 `/etc/lobster-chat/gateway.env` 不随 artifact 覆盖。
4. 只有新版本确实写入不兼容状态时才恢复旧状态备份；不要默认回滚用户数据。
5. 回滚后执行公网 smoke、真实 OTP 和双端 IM 验收。

## 9. 故障定位顺序

1. `systemctl status` / `journalctl`：Gateway 是否真实运行。
2. `curl 127.0.0.1:8787/health`：应用本体。
3. `nginx -t` 和本机 Nginx URL：反向代理。
4. 公网 `/health`：DNS、TLS、CDN、防火墙。
5. 邮件 Webhook 日志和真实收件箱：投递服务。

出站公网 IP 不证明入站流量会到达此主机。相关案例与区域网络注意事项见 [DEPLOYMENT_PITFALLS_AND_HARDENING.md](DEPLOYMENT_PITFALLS_AND_HARDENING.md)。

## 10. 明确后置范围

- 原生 Waku relay 跨城互联
- 真实 MLS 群组加密
- 链上锚定
- SMS OTP
- PWA/IndexedDB 离线同步
- 穿戴设备专用 transport bridge

这些项目不属于当前单城集中式 IM 的上线阻塞项。
