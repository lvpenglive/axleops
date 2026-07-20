# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.4.x   | Yes |
| < 0.4   | Best-effort（请尽快升级） |

## Reporting a vulnerability

请**不要**在公开 Issue 里贴可利用细节或真实 Token。

优先通过 GitHub 仓库的 **Security Advisories**（Security → Report a vulnerability）私下报告；若不可用，可向仓库维护者私信/邮件说明：

- 影响组件（admin / agent / proxy）
- 版本或提交
- 复现步骤与影响面（越权、泄露、RCE 等）

我们会确认后评估修复与披露节奏。

## Production checklist（生产上线前）

部署到非本机实验环境前，请确认：

### Secrets

- [ ] 修改所有示例口令与 Token（Admin `token` / `seed_password`、Agent `token`、Proxy `token`）
- [ ] `config.toml`、`data/`、`auth.token`、`.env` **不入库**、权限收紧（如 `chmod 600`）
- [ ] 定期轮换 Agent / Proxy Token（控制台「轮换 Token」）

### Network

- [ ] Admin / Agent / Proxy 仅监听必要网段（或前置反向代理 + TLS）
- [ ] 跨网区时 Admin 只通 Proxy，业务 Agent 不对公网暴露
- [ ] 生产配置 `cors_origins` 为具体前端来源（不要长期 `*` / 空宽松模式）

### Runtime

- [ ] 使用 release 二进制或经审核的安装包
- [ ] 优先 systemd / Windows 服务托管（见 `deploy/`）；容器 Agent 不适合管宿主机进程
- [ ] 为业务 JAR/脚本配置 `health_url`（如适用）
- [ ] 备份 Admin `data/axleops.db` 与关键配置

### Accounts

- [ ] 首次登录后立即改掉种子管理员密码
- [ ] 禁用不用的账号；自动化使用独立服务 Token，勿共享个人会话

### Offline / air-gapped

- [ ] 二进制与依赖来自内网可信构建或介质，而非临时公网下载
- [ ] 制品（JAR）经内网盘/制品库流转，再用 Admin/Agent 发布 API

## Security notes

- Agent 以运行用户权限启动子进程；管理 JAR/脚本路径时请最小授权。
- Admin 服务 Token 与浏览器会话分离；泄露 Token 等于控制面自动化权限。
- Proxy 持有下游 Agent Token；Proxy 主机与 `upstreams` 数据需重点防护。
