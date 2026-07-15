# AxleOps

微服务及其他服务启停管理：支持 JAR、shell/Python 等脚本与通用命令。

## 结构

| 目录 | 说明 |
|------|------|
| `axleops-agent/` | 机器端 Agent（进程启停、日志、探活）默认 `:9100` |
| `axleops-admin/` | 控制面（Agent 注册、Web 控制台、SQLite）默认 `:9000` |

## 快速开始

### Agent

```bash
cd axleops-agent
cp config.example.toml config.toml   # 修改 token
cargo run
```

### Admin

```bash
cd axleops-admin
cp config.example.toml config.toml   # 修改 token
cargo run
```

浏览器打开 http://127.0.0.1:9000/ ，用 Admin Token 登录后注册 Agent（如 `http://127.0.0.1:9100`）。

配置请使用 `config.example.toml` 复制为本地 `config.toml`（该文件不入库）。
