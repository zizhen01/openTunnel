# AGENTS.md — openTunnel (cft)

## Project Identity

**openTunnel** (`cft`) is an open-source CLI tool for managing Cloudflare Tunnels, DNS, Zero Trust Access, and monitoring — built in Rust.

- Binary name: `cft`
- License: MIT
- Goal: become the **best community-driven Cloudflare Tunnel management tool**
- Languages: bilingual (English + Chinese), switchable at runtime

---

## Architecture Overview

### Target Module Structure

```
src/
├── main.rs              # Entry point, CLI parsing, command dispatch
├── cli.rs               # Clap command/subcommand definitions
├── client.rs            # CloudflareClient — unified API wrapper
├── config.rs            # Config read/write (~/.cft/ and /etc/cloudflared/)
├── i18n.rs              # Bilingual text system (en/zh)
├── menu.rs              # Interactive menu (dialoguer)
├── tunnel.rs            # Tunnel CRUD (API + cloudflared CLI)
├── dns.rs               # DNS record management via API
├── access.rs            # Zero Trust / Access apps & policies
├── monitor.rs           # Stats, real-time monitor, Prometheus parsing
├── scan.rs              # Local service discovery & port scanning
├── tools.rs             # Health check, auto-fix, debug, export/import
└── error.rs             # Unified error types (thiserror + anyhow)
```

### Key Design Principles

1. **No stubs** — every menu item and CLI command must do real work or be removed
2. **Fail gracefully** — never `.unwrap()` on user input or I/O; use `anyhow::Result` everywhere
3. **One API client** — all Cloudflare API calls go through `CloudflareClient` in `client.rs`
4. **Config as source of truth** — read/write `/etc/cloudflared/config.yml` for tunnel config; `~/.cft/config.json` for API credentials
5. **Offline-safe** — features that don't need the API (list mappings, scan ports) must work without network

---

## Bilingual (i18n) System

### Requirements

- All user-facing strings (menus, prompts, errors, help text) must exist in **both English and Chinese**
- Language is selected via:
  1. `--lang en` / `--lang zh` CLI flag (highest priority)
  2. `CFT_LANG` environment variable
  3. `language` field in `~/.cft/config.json`
  4. System locale detection (`LANG` / `LC_ALL`) as fallback
  5. Default: `en`

### Implementation Pattern

Use a simple macro-based approach in `src/i18n.rs`:

```rust
// src/i18n.rs
pub enum Lang { En, Zh }

macro_rules! t {
    ($lang:expr, $en:expr, $zh:expr) => {
        match $lang {
            Lang::En => $en,
            Lang::Zh => $zh,
        }
    };
}

// Usage:
println!("{}", t!(lang, "Scanning local services...", "扫描本地服务..."));
```

### Rules

- Never hardcode Chinese-only or English-only user-facing strings
- `clap` help/about text: use the `about` and `long_about` attributes with both languages via a helper
- Error messages from `anyhow`/`thiserror`: use English internally, translate at display boundary
- Code comments: English only
- Commit messages: English only
- README: maintain two versions — `README.md` (English, primary) and `README_ZH.md` (Chinese)

---

## Coding Standards

### Rust Conventions

- Edition: 2021
- MSRV (Minimum Supported Rust Version): 1.75+
- Format: `cargo fmt` (rustfmt default settings)
- Lint: `cargo clippy -- -D warnings` must pass with zero warnings
- All public functions and structs must have doc comments (`///`)
- Prefer `thiserror` for library-style errors, `anyhow` for application-level propagation

### Error Handling

```rust
// NEVER do this:
let value = something().unwrap();

// Do this instead:
let value = something().context("failed to do something")?;

// For dialoguer interactions that can be cancelled:
let selection = Select::new()
    .interact_opt()?
    .ok_or_else(|| anyhow!("selection cancelled"))?;
```

### API Client Pattern

```rust
// All API calls must go through CloudflareClient
pub struct CloudflareClient {
    client: reqwest::Client,
    token: String,
    account_id: String,
    base_url: String,  // default: https://api.cloudflare.com/client/v4
}

impl CloudflareClient {
    pub async fn list_tunnels(&self) -> Result<Vec<Tunnel>> { ... }
    pub async fn list_dns_records(&self, zone_id: &str) -> Result<Vec<DnsRecord>> { ... }
    // etc.
}
```

### Security Rules

- **Never** log or print API tokens (mask as `cf_***...***`)
- Set file permissions to `0600` when writing `~/.cft/config.json`
- Validate all user input before passing to shell commands
- Use `std::process::Command` with explicit args (never shell interpolation)
- No `unsafe` blocks without a `// SAFETY:` comment and team review

### Testing

- Unit tests: in each module via `#[cfg(test)] mod tests`
- Integration tests: in `tests/` directory
- Use `mockito` or `wiremock` for API mocking
- Target: 70%+ code coverage for non-stub modules
- `cargo test` must pass before any PR is merged

---

## Git & Contribution Workflow

### Branch Strategy

- `main` — stable, always builds, always passes CI
- `dev` — integration branch for feature work
- `feat/<name>` — feature branches (e.g., `feat/dns-crud`)
- `fix/<name>` — bug fix branches
- `docs/<name>` — documentation changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(dns): implement DNS record CRUD via Cloudflare API
fix(monitor): handle connection timeout in real-time monitor
refactor(config): extract config module from main.rs
docs: add contributing guide
ci: add GitHub Actions workflow for clippy + test
i18n: add English translations for tunnel menu
```

### Pull Request Checklist

- [ ] `cargo fmt` — no formatting issues
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] All user-facing strings have both `en` and `zh` translations
- [ ] No `.unwrap()` on fallible operations (use `?` or `.context()`)
- [ ] New public APIs have doc comments
- [ ] README updated if user-facing behavior changed
- [ ] No hardcoded paths that break cross-platform (use `dirs` crate for home directory)

---

## Implementation Priority

### Phase 1 — Foundation (make existing features real)

1. **Modularize** — split `main.rs` into the module structure above
2. **Error handling** — replace all `.unwrap()` with proper error propagation
3. **i18n system** — implement `Lang` enum + `t!` macro, convert all strings
4. **Real system status** — detect `cloudflared` service, read actual config
5. **Config read/write** — parse and modify `/etc/cloudflared/config.yml`
6. **CloudflareClient** — unified API client with token, account_id, zone_id

### Phase 2 — Core Features

7. **Tunnel CRUD** — list/create/delete/switch tunnels (API + `cloudflared` CLI)
8. **Domain mapping** — add/remove ingress rules in config, restart service
9. **DNS CRUD** — full DNS record management via Cloudflare API
10. **Service control** — start/stop/restart via `systemctl` (Linux) or `launchctl` (macOS)
11. **Port scan improvements** — custom port ranges, configurable timeout, process name detection

### Phase 3 — Advanced

12. **Zero Trust / Access** — app CRUD, policy management, user listing
13. **Monitoring** — historical metrics, alerting thresholds, better Prometheus parsing
14. **Config export/import** — JSON/YAML dump, cross-machine sync
15. **Template system** — predefined configs for common setups (React + API, WordPress, etc.)

### Phase 4 — Polish

16. **Cross-platform** — macOS `launchctl` support, Windows service support
17. **Shell completions** — `clap_complete` for bash/zsh/fish
18. **Auto-update** — self-update mechanism
19. **Plugin system** — extensible command framework
20. **TUI dashboard** — `ratatui`-based real-time dashboard (optional)

---

## Dependency Policy

| Category | Crate | Purpose |
|----------|-------|---------|
| CLI | `clap` 4.x (derive) | Argument parsing |
| CLI | `dialoguer` 0.11.x | Interactive prompts |
| CLI | `clap_complete` | Shell completions |
| Display | `colored` 2.x | Terminal colors |
| Display | `comfy-table` 7.x | ASCII tables |
| Serialization | `serde` 1.x | Serialize/Deserialize |
| Serialization | `serde_json` 1.x | JSON config |
| Serialization | `serde_yaml` 0.9.x | YAML tunnel config |
| HTTP | `reqwest` 0.12.x | Cloudflare API calls |
| Async | `tokio` 1.x (full) | Async runtime |
| Error | `anyhow` 1.x | Application errors |
| Error | `thiserror` 1.x | Typed errors |
| Time | `chrono` 0.4.x | Timestamps |
| Paths | `dirs` 5.x | Cross-platform home dir |
| Logging | `tracing` + `tracing-subscriber` | Structured logging |
| Testing | `wiremock` | HTTP API mocking |

Do **not** add dependencies without justification. Prefer stdlib when feasible.

---

## File & Config Paths

| Path | Purpose | Platform |
|------|---------|----------|
| `~/.cft/config.json` | API credentials, preferences, language | All |
| `/etc/cloudflared/config.yml` | Tunnel ingress config (Linux) | Linux |
| `~/Library/Application Support/cloudflared/config.yml` | Tunnel config (macOS) | macOS |
| `~/.cloudflared/` | Cloudflared credentials | All |

Always use `dirs::home_dir()` or `dirs::config_dir()` — never hardcode `/root` or `$HOME`.

---

## CI/CD (GitHub Actions)

Required workflows:

```yaml
# .github/workflows/ci.yml
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test
- cargo build --release (linux x86_64, aarch64, macOS, Windows)
```

Release workflow:
- Tag-triggered (`v*`)
- Build binaries for all platforms
- Publish to GitHub Releases
- Optional: publish to crates.io

---

## Open Source Standards

### Required Project Files

- [ ] `README.md` — English (primary, with badges and install instructions)
- [ ] `README_ZH.md` — Chinese (linked from English README)
- [ ] `LICENSE` — MIT
- [ ] `CONTRIBUTING.md` — how to contribute (bilingual)
- [ ] `CHANGELOG.md` — keep-a-changelog format
- [ ] `AGENTS.md` — this file
- [ ] `.github/ISSUE_TEMPLATE/` — bug report + feature request templates
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] `.github/workflows/ci.yml`

### Quality Badges (for README)

```markdown
[![CI](https://github.com/user/openTunnel/actions/workflows/ci.yml/badge.svg)]
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)]
[![Crates.io](https://img.shields.io/crates/v/cft.svg)]
```

---

## Agent Instructions

When working on this codebase as an AI agent:

1. **Read before write** — always read existing code before modifying
2. **One module at a time** — complete one module fully before starting the next
3. **No dead code** — if a feature isn't implemented, don't leave a stub; remove the menu entry until it's ready
4. **Test what you build** — write tests alongside implementation, not after
5. **Bilingual always** — every new user-facing string needs both `en` and `zh`
6. **Security first** — never expose tokens, always validate input, set file permissions
7. **Small PRs** — one feature or fix per PR, with clear description
8. **Run checks** — `cargo fmt && cargo clippy -- -D warnings && cargo test` before declaring done
