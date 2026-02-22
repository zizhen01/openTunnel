# openTunnel (`tunnel`)

一个开源的 [cloudflared](https://github.com/cloudflare/cloudflared) 封装工具，把 Cloudflare Tunnel 管理变成一个交互式 CLI。

`tunnel` 覆盖完整生命周期 —— 从自动安装 cloudflared、创建隧道、映射域名、配置 DNS、管理 Zero Trust Access，到运行系统服务 —— 全部通过交互菜单或脚本化子命令完成。

[English README](./README.md)

## 特性

- **cloudflared 全生命周期管理** —— 未安装时自动下载安装（Linux/macOS/Windows），安装/启动/重启系统服务
- **一键域名映射** —— 一步完成 `app.example.com → localhost:3000`，自动创建 DNS 记录
- **完整隧道操作** —— 创建、列出、删除隧道；添加/移除/查看 ingress 映射
- **DNS 管理** —— 列出/添加/删除记录；隧道路由自动同步到 DNS
- **Zero Trust Access** —— 创建/删除应用，管理访问策略
- **服务控制** —— 安装、启动、停止、重启 cloudflared 服务；查看日志
- **监控诊断** —— 健康检查、隧道统计、实时监控、本地端口扫描
- **中英双语** —— 运行时随时切换

## 安装

### 从源码安装

```bash
cargo install --path .
```

### 或手动编译

```bash
cargo build --release
cp target/release/tunnel /usr/local/bin/
```

## 快速上手

```bash
# 1. 配置 Cloudflare API Token
tunnel config set

# 2. 启动交互菜单
tunnel

# 3. 或直接使用子命令
tunnel list                          # 查看隧道
tunnel map app.example.com http://localhost:3000   # 映射域名
tunnel dns sync --tunnel <ID>        # 同步 DNS 记录
tunnel service install --tunnel <ID> # 安装并启动 cloudflared 服务
tunnel scan                          # 发现本地服务
```

## 命令参考

### 隧道

| 命令 | 说明 |
|------|------|
| `tunnel list` | 列出所有隧道 |
| `tunnel create [name]` | 创建新隧道 |
| `tunnel delete` | 删除隧道（交互选择） |
| `tunnel token [id]` | 获取隧道运行 Token |

### 域名映射

| 命令 | 说明 |
|------|------|
| `tunnel map [hostname] [service]` | 添加域名映射（如 `app.example.com http://localhost:3000`） |
| `tunnel unmap [hostname]` | 移除域名映射 |
| `tunnel show [id]` | 查看当前映射 |

### DNS

| 命令 | 说明 |
|------|------|
| `tunnel dns list` | 列出 DNS 记录 |
| `tunnel dns add` | 添加 DNS 记录 |
| `tunnel dns delete [id]` | 删除 DNS 记录 |
| `tunnel dns sync --tunnel <id>` | 同步隧道路由到 DNS |

### Zero Trust Access

| 命令 | 说明 |
|------|------|
| `tunnel access list` | 列出 Access 应用 |
| `tunnel access create [name] --domain <domain>` | 创建 Access 应用 |
| `tunnel access delete [id]` | 删除 Access 应用 |
| `tunnel access policy [app_id]` | 管理访问策略 |

### 服务管理（cloudflared）

| 命令 | 说明 |
|------|------|
| `tunnel service status` | 查看服务状态 |
| `tunnel service install --tunnel <id>` | 为隧道安装服务 |
| `tunnel service start` | 启动服务 |
| `tunnel service stop` | 停止服务 |
| `tunnel service restart` | 重启服务 |
| `tunnel service logs` | 查看最近日志 |

### 配置

| 命令 | 说明 |
|------|------|
| `tunnel config set` | 交互式配置向导 |
| `tunnel config show` | 查看当前配置 |
| `tunnel config test` | 测试 API 连接 |
| `tunnel config lang en\|zh` | 设置语言 |

### 实用工具

| 命令 | 说明 |
|------|------|
| `tunnel scan` | 扫描本地服务 |
| `tunnel`（无参数） | 进入交互菜单 |

## 工作原理

```
┌──────────────┐     Cloudflare API      ┌─────────────────────┐
│  tunnel CLI  │ ──────────────────────── │  Cloudflare Edge    │
│  (本工具)     │  (隧道/DNS/Access)       │  ── 隧道路由         │
└──────┬───────┘                          │  ── DNS 记录         │
       │                                  │  ── Access 策略      │
       │  管理                             └─────────────────────┘
       ▼
┌──────────────┐     cloudflared tunnel   ┌─────────────────────┐
│  cloudflared │ ════════════════════════ │  你的本地服务         │
│  (系统服务)   │     (持久连接)            │  :3000, :8080 等    │
└──────────────┘                          └─────────────────────┘
```

`tunnel` 通过 Cloudflare API 管理配置（隧道、DNS、Access），并将 cloudflared 作为系统服务管理，负责实际的流量代理。

## 语言设置

优先级：

1. `--lang en|zh`
2. `CFT_LANG` 环境变量
3. 已保存配置（`~/.cft/config.json`）
4. 系统语言环境

## 常用路径

| 路径 | 用途 |
|------|------|
| `~/.cft/config.json` | API Token、Account/Zone ID、语言设置 |
| `/etc/cloudflared/config.yml` | 隧道配置（Linux） |
| `~/.cloudflared/config.yml` | 隧道配置（macOS） |

## 贡献

欢迎提 Issue 和 PR。提交前请运行：

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 许可证

MIT
