## Description
<!-- Please include a summary of the change and which issue is fixed -->

Fixes PLO-XX (issue)

## Type of Change
<!-- Please delete options that are not relevant -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Refactoring

## Checklist
<!-- Please check all that apply -->

- [ ] `cargo fmt` — no formatting issues
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] All user-facing strings have both `en` and `zh` translations
- [ ] No `.unwrap()` on fallible operations (use `?` or `.context()`)
- [ ] New public APIs have doc comments
- [ ] README updated if user-facing behavior changed
- [ ] No hardcoded paths that break cross-platform (use `dirs` crate for home directory)
- [ ] **Every commit references a Linear task ID** (e.g., `Fixes PLO-34`)
- [ ] CHANGELOG.md updated (if applicable)

## Testing
<!-- Please describe the tests that you ran to verify your changes -->

- [ ] Tested manually
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated

## Screenshots (if applicable)

## Additional Context
<!-- Add any other context about the PR here -->
