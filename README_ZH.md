# opentunnel (`tunnel`)

一个面向社区的 Cloudflare Tunnel 管理 CLI，覆盖隧道、DNS、Zero Trust Access 与监控。

`tunnel` 的目标是日常可用：上手快、交互安全、排障直接。

[English README](./README.md)

## 为什么用 `tunnel`

- 一个 CLI 处理 Tunnel / DNS / Access / 监控
- 交互模式更安全（`tunnel`）
- 也支持脚本化子命令
- 中英双语运行时切换
- 本地能力离线可用（配置查看、端口扫描等）

## 安装

```bash
cargo build --release
cp target/release/tunnel /usr/local/bin/
```

也可以直接运行：

```bash
cargo run -- <command>
```

## 30 秒上手

```bash
# 1) 配置 API Token / Account / Zone
tunnel config set

# 2) 进入交互菜单
tunnel

# 3) 试几个核心能力
tunnel list
tunnel dns list
tunnel scan
```

## 核心能力

- 隧道管理：列出、创建、切换、删除
- 映射管理：添加/移除/查看本地服务映射
- DNS 管理：列出/添加/删除，隧道路由同步到 DNS
- Zero Trust Access：应用增删查、策略管理
- 服务控制：启动/停止/重启/状态
- 诊断监控：健康检查、统计、实时监控、调试信息

## 语言设置

优先级如下：

1. `--lang en|zh`
2. `CFT_LANG`
3. 已保存配置（`~/.cft/config.json`）
4. 系统语言环境

示例：

```bash
tunnel --lang zh
tunnel config lang en
```

## 常用路径

- API 配置：`~/.cft/config.json`
- 隧道配置（Linux）：`/etc/cloudflared/config.yml`
- 隧道配置（macOS）：`~/.cloudflared/config.yml`

## 项目状态

`tunnel` 正在持续迭代，当前重点是稳定性、真实 API 行为和双语体验。

## 贡献

欢迎提 Issue 和 PR。

建议提交前运行：

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 许可证

MIT
