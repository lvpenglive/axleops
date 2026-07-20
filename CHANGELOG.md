# Changelog

本文件记录 AxleOps 面向用户的显著变更。版本号大致遵循 [SemVer](https://semver.org/)。

格式参考 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

### Planned

- 告警（钉钉 / 企微）
- 依赖图与顺序启停
- 细粒度 RBAC
- Admin 侧规格镜像、多环境标签

## [0.4.0] - 2026-07-20

### Added

- 制品上传、版本列表、发布（停 → 换包 → 启）与回滚（Agent `artifacts` / `publish` / `rollback`）
- Linux systemd 与 Windows 服务/计划任务安装脚本（`deploy/linux`、`deploy/windows`）
- Docker Compose 一键联调（`docker-compose.yml`、`deploy/docker`）
- Admin 控制台：制品发布区、卡片折叠、表单高级选项、服务与制品联动

### Changed

- 发布包说明中补充服务安装与 Docker 用法
- README 路线图将 v0.4 标为完成

## [0.3.0] - 2026-07-20

### Added

- Admin 用户名/密码登录、会话鉴权、改密、用户管理、操作审计
- 跨 Agent 服务总览
- Agent / Proxy Token 轮换（Agent 支持热更与 `data/auth.token`）
- Admin `cors_origins` 生产 CORS 策略
- Agent 看门狗与 `desired_running` 恢复
- CI `cargo test`（agent / admin）

### Changed

- 浏览器走会话；自动化仍可用 `X-AxleOps-Token`
- 启动探活与进程强杀策略可配置

## [0.2.0] - 2026-07

### Added

- Agent / Proxy 在线态与指示灯
- 日志自动刷新与尾部跟随
- 同机批量启停 / 重启
- Proxy 管理页与上游导入为 Agent

### Changed

- 表单与代理错误提示增强
- 登录页与控制台 UX 打磨

## [0.1.0] - 2026-07

### Added

- Agent：JAR / 脚本 / 命令启停、规格持久化、日志、env、health 探活
- Admin：Token 与控制台、Agent 注册、API 代理
- Proxy：边界转发与上游列表
- `config.toml` + 环境变量；运行数据不入库

[Unreleased]: https://github.com/lvpenglive/axleops/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/lvpenglive/axleops/releases/tag/v0.4.0
[0.3.0]: https://github.com/lvpenglive/axleops/releases/tag/v0.3.0
[0.2.0]: https://github.com/lvpenglive/axleops/releases/tag/v0.2.0
[0.1.0]: https://github.com/lvpenglive/axleops/releases/tag/v0.1.0
