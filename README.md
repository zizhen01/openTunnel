# openTunnel (`tunnel`)

An open-source wrapper around [cloudflared](https://github.com/cloudflare/cloudflared) that turns Cloudflare Tunnel management into a single interactive CLI.

`tunnel` handles the full lifecycle — from installing cloudflared itself, to creating tunnels, mapping domains, configuring DNS, managing Zero Trust Access, and running the service — all through an interactive menu or scriptable subcommands.

[中文文档](./README_ZH.md)

## Features

- **cloudflared lifecycle management** — auto-installs cloudflared if missing (Linux/macOS/Windows), installs/starts/restarts the system service
- **Quick domain mapping** — map `app.example.com → localhost:3000` in one step, with automatic DNS record creation
- **Full tunnel operations** — create, list, delete tunnels; add/remove/show ingress mappings
- **DNS management** — list, add, delete records; sync tunnel routes to DNS automatically
- **Zero Trust Access** — create/delete applications, manage access policies
- **Service control** — install, start, stop, restart cloudflared service; view logs
- **Monitoring** — health check, tunnel stats, real-time monitor, local port scanner
- **Bilingual** — English and Chinese, switchable at runtime

## Install

### From source

```bash
cargo install --path .
```

### Or build manually

```bash
cargo build --release
cp target/release/tunnel /usr/local/bin/
```

## Quick start

```bash
# 1. Configure your Cloudflare API token
tunnel config set

# 2. Launch interactive menu
tunnel

# 3. Or use subcommands directly
tunnel list                          # list tunnels
tunnel map app.example.com http://localhost:3000   # map a domain
tunnel dns sync --tunnel <ID>        # sync DNS records
tunnel service install --tunnel <ID> # install & start cloudflared service
tunnel scan                          # discover local services
```

## CLI reference

### Tunnel

| Command | Description |
|---------|-------------|
| `tunnel list` | List all tunnels |
| `tunnel create [name]` | Create a new tunnel |
| `tunnel delete` | Delete a tunnel (interactive) |
| `tunnel token [id]` | Get tunnel run token |

### Domain mapping

| Command | Description |
|---------|-------------|
| `tunnel map [hostname] [service]` | Add domain mapping (e.g. `app.example.com http://localhost:3000`) |
| `tunnel unmap [hostname]` | Remove domain mapping |
| `tunnel show [id]` | Show current mappings |

### DNS

| Command | Description |
|---------|-------------|
| `tunnel dns list` | List DNS records |
| `tunnel dns add` | Add a DNS record |
| `tunnel dns delete [id]` | Delete a DNS record |
| `tunnel dns sync --tunnel <id>` | Sync tunnel routes to DNS |

### Zero Trust Access

| Command | Description |
|---------|-------------|
| `tunnel access list` | List Access applications |
| `tunnel access create [name] --domain <domain>` | Create Access application |
| `tunnel access delete [id]` | Delete Access application |
| `tunnel access policy [app_id]` | Manage access policies |

### Service (cloudflared)

| Command | Description |
|---------|-------------|
| `tunnel service status` | Show service status |
| `tunnel service install --tunnel <id>` | Install service for a tunnel |
| `tunnel service start` | Start service |
| `tunnel service stop` | Stop service |
| `tunnel service restart` | Restart service |
| `tunnel service logs` | Show recent logs |

### Config

| Command | Description |
|---------|-------------|
| `tunnel config set` | Interactive setup wizard |
| `tunnel config show` | Show current configuration |
| `tunnel config test` | Test API connection |
| `tunnel config lang en\|zh` | Set language |

### Utilities

| Command | Description |
|---------|-------------|
| `tunnel scan` | Scan local services |
| `tunnel` (no args) | Interactive menu |

## How it works

```
┌──────────────┐     Cloudflare API      ┌─────────────────────┐
│  tunnel CLI  │ ──────────────────────── │  Cloudflare Edge    │
│  (this tool) │     (tunnel/dns/access)  │  ── Tunnel routing  │
└──────┬───────┘                          │  ── DNS records     │
       │                                  │  ── Access policies │
       │  manages                         └─────────────────────┘
       ▼
┌──────────────┐     cloudflared tunnel   ┌─────────────────────┐
│  cloudflared │ ════════════════════════ │  Your local services│
│  (service)   │     (persistent conn)    │  :3000, :8080, etc. │
└──────────────┘                          └─────────────────────┘
```

`tunnel` talks to the Cloudflare API for configuration (tunnels, DNS, Access) and manages cloudflared as a system service for the actual traffic proxying.

## Language

Set language by priority:

1. `--lang en|zh`
2. `CFT_LANG` environment variable
3. Saved config (`~/.cft/config.json`)
4. System locale

## Paths

| Path | Purpose |
|------|---------|
| `~/.cft/config.json` | API token, account/zone IDs, language |
| `/etc/cloudflared/config.yml` | Tunnel config (Linux) |
| `~/.cloudflared/config.yml` | Tunnel config (macOS) |

## Contributing

Issues and PRs welcome. Before submitting:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## License

MIT
