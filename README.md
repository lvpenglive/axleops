# AxleOps

微服务及其他服务启停管理：支持 JAR、shell / Python 等脚本和通用命令。

- Agent：机器端进程生命周期（启停、日志、探活）
- Admin：控制面 + Web 控制台 + SQLite Agent 注册

仓库：https://github.com/lvpenglive/axleops

## 结构

| 目录 | 说明 |
|------|------|
| `axleops-agent/` | 机器端 Agent，默认 `0.0.0.0:9100` |
| `axleops-admin/` | 控制面与静态 UI，默认 `0.0.0.0:9000` |

## 版本功能计划

当前基线为 **v0.1.0**。

### v0.1 — 基线（已完成）

- Agent：JAR / 脚本 / 命令启停，规格持久化，日志，env，health 探活，重启 / 删除
- Admin：Token 登录，Agent 注册，API 代理，Web 控制台
- 配置：`config.toml` + 环境变量覆盖；运行数据与密钥不入库

### v0.2 — 控制台打磨（下一版）

- 日志 UX：自动刷新、尾部跟随、行数筛选
- Agent 在线态：心跳或定时 ping，列表显示 online / offline / 延迟
- 同机批量启停 / 重启
- 表单与代理错误提示增强
- README / `config.example.toml` 与实现字段对齐维护

### v0.3 — 可靠与安全

- Token 轮换、可选按 Agent 独立 token
- 生产绑定与 CORS 策略
- 跨 Agent 服务状态总览
- 操作审计（谁、何时、对哪台做了启停）
- Agent 重启后按规格拉起「应运行」服务
- 基础自动化测试（进程生命周期 + 代理冒烟）

### v0.4 — 交付与部署增强

- 制品上传 / 版本号（JAR 换包后启停）
- 简单发布：停 → 换文件 → 启；可选回滚
- Windows 服务 / Linux systemd 安装脚本
- 可选 Docker / 一键安装包

### v0.5+ — 远期

依赖图与顺序启停、告警（钉钉 / 企微）、RBAC、Admin 侧规格镜像、多环境（dev / staging / prod）

**建议落地顺序：** v0.2 → v0.3（心跳 + 开机拉起）→ 有换包需求再开 v0.4。

---

## 实施与部署步骤

### 0. 环境要求

- Rust 工具链（建议 stable）：https://rustup.rs
- Windows / Linux 均可；业务机需能执行目标 JAR / 脚本 / 命令
- Admin 与各 Agent 网络互通（默认 Admin `9000`，Agent `9100`）

### 1. 获取代码

```bash
git clone https://github.com/lvpenglive/axleops.git
cd axleops
```

### 2. 配置（务必修改 Token）

本地 `config.toml` **不入库**，从示例复制：

**Agent（每台业务机一份）：**

```bash
cd axleops-agent
cp config.example.toml config.toml
```

```toml
bind = "0.0.0.0:9100"
token = "改成足够长的随机密钥"   # 请求头 X-AxleOps-Token
data_dir = "./data"
```

环境变量覆盖（可选）：`AXLEOPS_BIND` / `AXLEOPS_TOKEN` / `AXLEOPS_DATA_DIR`

**Admin（一台控制机）：**

```bash
cd axleops-admin
cp config.example.toml config.toml
```

```toml
bind = "0.0.0.0:9000"
token = "改成足够长的随机密钥"   # Web 登录与 API 使用同一 Token
data_dir = "./data"
```

环境变量覆盖（可选）：`AXLEOPS_ADMIN_BIND` / `AXLEOPS_ADMIN_TOKEN` / `AXLEOPS_ADMIN_DATA_DIR`

> Admin Token 与 Agent Token 当前各自独立配置。在控制台注册 Agent 时，需填写该 Agent 的 `base_url` 与其 Token，以便 Admin 代理请求。

### 3. 开发态启动（本机联调）

终端 1 — Agent：

```bash
cd axleops-agent
cargo run
```

终端 2 — Admin：

```bash
cd axleops-admin
cargo run
```

1. 浏览器打开 http://127.0.0.1:9000/
2. 使用 Admin `config.toml` 中的 `token` 登录
3. 注册 Agent：名称 + `http://127.0.0.1:9100` + Agent Token
4. 进入该 Agent，保存服务规格（JAR / 脚本 / 命令），再启动 / 看日志 / 探活

### 4. 生产 / 联机部署（推荐 release）

在**构建机**上编译（或各机分别编译）：

```bash
# Agent
cd axleops-agent
cargo build --release
# 产物：target/release/axleops-agent  （Windows: axleops-agent.exe）

# Admin
cd ../axleops-admin
cargo build --release
# 产物：target/release/axleops-admin  （Windows: axleops-admin.exe）
```

**部署目录建议（每台机器）：**

```text
/opt/axleops/agent/          # 或 E:\axleops\agent\
  axleops-agent(.exe)
  config.toml
  data/                      # 自动创建：services / pids / logs

/opt/axleops/admin/          # 仅控制机
  axleops-admin(.exe)
  config.toml
  static/                    # 需与二进制一起部署（UI）
  data/                      # SQLite：axleops.db
```

注意：

- Admin 的静态页面在 `axleops-admin/static/`，release 运行时工作目录需能找到该目录（或在 `config` / 代码约定路径下放置），建议把 `static` 与可执行文件放在同一部署包内，并从该目录启动进程。
- `data/` 含运行态数据，备份时一并备份；不要把含真实 Token 的 `config.toml` 提交到 Git。

**启动示例：**

```bash
# Linux / macOS
cd /opt/axleops/agent && ./axleops-agent
cd /opt/axleops/admin && ./axleops-admin

# Windows（PowerShell）
cd E:\axleops\agent; .\axleops-agent.exe
cd E:\axleops\admin; .\axleops-admin.exe
```

### 5. 多机组网清单

| 步骤 | 操作 |
|------|------|
| 1 | 每台业务机部署 Agent，开放 `9100`（或自定义端口）给 Admin 所在网段 |
| 2 | 控制机部署 Admin，开放 `9000` 给运维浏览器 |
| 3 | 各 Agent `token` 独立、足够随机；防火墙仅放行必要源 IP |
| 4 | 在 Admin 控制台逐台注册：`http://<业务机IP>:9100` + 对应 Token |
| 5 | 先 `ping` / 列表服务，再配置业务 JAR 的绝对路径与 `work_dir` |
| 6 | 需要探活时填写 `health_url`（如 Spring Actuator） |

跨机时 Agent 的 `base_url` 必须用业务机真实 IP/主机名，不能写 `127.0.0.1`（那是 Admin 本机环回）。

### 6. 校验与日常操作

```bash
# Agent 存活
curl http://<agent-host>:9100/health

# Admin 存活
curl http://<admin-host>:9000/health
```

日常：登录 Admin → 选 Agent → 保存 / 启动 / 停止 / 重启 / 删服务 / 看日志。  
改 Agent / Admin 代码后需重新 `cargo build --release`（或 `cargo run`）并重启进程；仅改 `static/` 前端时重启 Admin 并强制刷新浏览器即可。

### 7. 升级（当前）

1. `git pull` 拿到新代码  
2. 两端 `cargo build --release`  
3. 停旧进程 → 替换二进制（Admin 同步更新 `static/`）→ 启新进程  
4. `config.toml` / `data/` 一般保留；若配置项有变更，对照新的 `config.example.toml` 合并  

开机自启、systemd / Windows 服务安装脚本规划在 **v0.4**。

---

## 配置说明摘要

| 组件 | 配置文件 | 默认端口 | 认证头 |
|------|----------|----------|--------|
| Agent | `axleops-agent/config.toml` | 9100 | `X-AxleOps-Token` |
| Admin | `axleops-admin/config.toml` | 9000 | `X-AxleOps-Token`（登录与 API） |

数据目录默认均为 `./data`（相对进程工作目录）。
