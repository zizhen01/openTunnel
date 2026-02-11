# opentunnel (`tunnel`)

A fast, community-driven CLI for managing Cloudflare Tunnels, DNS, and Zero Trust Access.

`tunnel` is designed for daily operations: quick setup, safe interactive workflows, and practical diagnostics.

[中文文档](./README_ZH.md)

## Why `tunnel`

- One CLI for tunnel, DNS, Access, and monitoring tasks
- Interactive mode for safer operations (`tunnel`)
- Script-friendly subcommands for automation
- Bilingual UX (English + Chinese)
- Works offline for local-only tasks (config inspection, service scan)

## Install

```bash
cargo build --release
cp target/release/tunnel /usr/local/bin/
```

Or run without installing:

```bash
cargo run -- <command>
```

## 30-second start

```bash
# 1) Configure API token/account/zone
tunnel config set

# 2) Open interactive menu
tunnel

# 3) Try a few core actions
tunnel list
tunnel dns list
tunnel scan
```

## What it can do

- Tunnel lifecycle: list, create, switch, delete
- Ingress mapping: add/remove/show local service mappings
- DNS records: list/add/delete, sync tunnel routes to DNS
- Zero Trust Access: list/create/delete apps, manage policies
- Service control: start/stop/restart/status
- Observability: health check, stats, real-time monitor, debug export

## Language

Set language by priority:

1. `--lang en|zh`
2. `CFT_LANG`
3. saved config (`~/.cft/config.json`)
4. system locale

Example:

```bash
tunnel --lang zh
tunnel config lang en
```

## Paths

- API config: `~/.cft/config.json`
- Tunnel config (Linux): `/etc/cloudflared/config.yml`
- Tunnel config (macOS): `~/.cloudflared/config.yml`

## Project status

`tunnel` is actively evolving toward a complete Cloudflare Tunnel operations toolkit. Current focus is reliability, real API behavior, and a clean bilingual experience.

## Contributing

Issues and PRs are welcome.

- Keep changes focused and small
- Run checks before PR:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## License

MIT
