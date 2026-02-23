# Contributing to openTunnel

Thank you for your interest in contributing to openTunnel! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/openTunnel.git`
3. Create a feature branch: `git checkout -b feat/PLO-XX-short-desc`

## Development Setup

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt

# Run clippy
cargo clippy -- -D warnings
```

## Commit Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/) and **require** every commit to reference a Linear task:

```
feat(dns): add DNS record deletion support

- Implement delete_record() in client.rs
- Add CLI command `dns delete`

Fixes PLO-12
```

**Important:** Every commit MUST correspond to exactly one Linear task. Create a Linear task before committing if one doesn't exist.

## Pull Request Process

1. Ensure all checks pass: `cargo fmt && cargo clippy -- -D warnings && cargo test`
2. Update CHANGELOG.md if applicable
3. Update README.md if user-facing behavior changed
4. Ensure all user-facing strings have both English and Chinese translations
5. Link the PR to the Linear task(s) it addresses

## Code Standards

- Use `anyhow::Result` for error handling
- Never use `.unwrap()` on fallible operations
- All public APIs must have doc comments
- All user-facing strings must support both English and Chinese

## Reporting Issues

Please use the GitHub issue templates when reporting bugs or requesting features.

---

# 贡献指南 (中文)

感谢您对 openTunnel 项目的关注！本文档提供了参与该项目的指南。

## 开始之前

1. Fork 仓库
2. 克隆您的 fork: `git clone https://github.com/yourusername/openTunnel.git`
3. 创建功能分支: `git checkout -b feat/PLO-XX-short-desc`

## 开发环境设置

```bash
# 构建项目
cargo build --release

# 运行测试
cargo test

# 检查格式化
cargo fmt

# 运行 clippy
cargo clippy -- -D warnings
```

## 提交规范

我们遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范，并**要求**每个 commit 都引用 Linear 任务：

```
feat(dns): add DNS record deletion support

- Implement delete_record() in client.rs
- Add CLI command `dns delete`

Fixes PLO-12
```

**重要：** 每个 commit 必须对应一个 Linear 任务。如果不存在，请先创建 Linear 任务再提交。

## Pull Request 流程

1. 确保所有检查通过: `cargo fmt && cargo clippy -- -D warnings && cargo test`
2. 如有需要，更新 CHANGELOG.md
3. 如影响用户行为，更新 README.md
4. 确保所有面向用户的字符串都有英文和中文翻译
5. 将 PR 链接到相关的 Linear 任务

## 代码规范

- 使用 `anyhow::Result` 进行错误处理
- 绝不在可能失败的操作上使用 `.unwrap()`
- 所有公共 API 必须有文档注释
- 所有面向用户的字符串必须支持英文和中文

## 报告问题

报告错误或请求功能时，请使用 GitHub issue 模板。
