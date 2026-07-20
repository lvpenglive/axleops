# Contributing

感谢关注 AxleOps。欢迎 Issue、讨论与 Pull Request。

## 开发环境

- Rust stable（[rustup](https://rustup.rs)）
- 可选：Docker（Compose 联调）

本机构建：

```bash
cd axleops-agent && cargo build
cd ../axleops-admin && cargo build
cd ../axleops-proxy && cargo build
```

测试：

```bash
cd axleops-agent && cargo test
cd ../axleops-admin && cargo test
```

联调：各组件目录复制 `config.example.toml` → `config.toml`，改掉默认密钥后 `cargo run`。  
或使用 `./deploy/docker/up.sh`（需 Docker）。

## 提交建议

- 变更尽量小而清晰；UI 与行为变更请附简要说明或截图
- 提交说明用英文或中文均可，说明「为什么」优先于堆砌文件名
- 涉及配置字段时，同步 `config.example.toml` 与 README
- 不要提交 `config.toml`、`data/`、真实 Token、本地 `.env`

## Pull Request

1. Fork 并基于最新 `main` 开分支
2. 本地 `cargo test`（至少改动的 crate）通过
3. PR 描述：动机、用法、测试方式
4. 安全相关改动请参考 [SECURITY.md](SECURITY.md)

## 行为准则

请保持友善、就事论事。恶意利用、骚扰或故意破坏社区的行为不可接受。

## 许可证

贡献内容默认按仓库 [MIT License](LICENSE) 授权。
