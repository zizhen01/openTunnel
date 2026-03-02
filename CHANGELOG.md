# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.9] - 2026-03-02

### Added
- Zone Settings menu under DNS Management: toggle "Always Use HTTPS" (HTTP → HTTPS redirect) via Cloudflare API
- Shell completions support (`tunnel completions bash|zsh|fish|elvish|powershell`)
- TUI Dashboard (`tunnel dashboard` or via menu) with real-time metrics, sparklines, and keybindings

### Fixed
- Access policy creation (code 12130): `PolicyRule` fields now omit `null` keys when serializing to JSON (`skip_serializing_if = "Option::is_none"`)

### Changed
- Removed `AGENTS.md` (AI agent instructions) from repository
- Automated `cargo publish` on git tag via GitHub Actions

## [0.1.8] - 2026-02-22

### Added
- Auto-install cloudflared when not present
- Cross-platform cloudflared service management
- UX improvements: auto-start, single DNS sync, menu cleanup
- Bilingual README refresh

## [0.1.4] - 2026-02-22

### Added
- CI release workflow with multi-platform builds
- API-driven tunnel workflows and account management

## [0.1.0] - 2026-02-22

### Added
- Initial release
- Tunnel CRUD operations
- Domain mapping management
- DNS record management
- Zero Trust Access application and policy management
- Service control (start/stop/restart)
- Local port scanning
- Real-time monitoring
- Bilingual support (English/Chinese)

[unreleased]: https://github.com/zizhen01/openTunnel/compare/v0.1.9...HEAD
[0.1.9]: https://github.com/zizhen01/openTunnel/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/zizhen01/openTunnel/compare/v0.1.4...v0.1.8
[0.1.4]: https://github.com/zizhen01/openTunnel/compare/v0.1.0...v0.1.4
[0.1.0]: https://github.com/zizhen01/openTunnel/releases/tag/v0.1.0
