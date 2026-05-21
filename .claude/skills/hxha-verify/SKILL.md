---
name: hxha-verify
description: Verify repository changes with the smallest sufficient project-specific formatting, linting, build, and test commands before claiming work is complete.
---

# HXHA Verify

Use this skill before saying code or configuration changes in this repository are complete.

## Required verification flow

1. Read this repository's `CLAUDE.md` and use its commands first.
2. If the repository is a Rust crate or workspace, prefer this order:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For a single-crate Rust project without a workspace, use:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

3. If the repository is Android/Gradle, use the Gradle wrapper and verification commands documented in `CLAUDE.md`.
4. If only Claude configuration files changed, validate JSON and file presence instead of running expensive app tests.

## Targeted tests

During development, run the smallest targeted command that covers the changed area before the full gate. Examples:

```bash
cargo test <test_name>
cargo test -p <package_name>
cargo check --workspace
```

## Completion rule

Only claim completion after reporting the exact commands run and whether they passed. If a command was not run, state why.
