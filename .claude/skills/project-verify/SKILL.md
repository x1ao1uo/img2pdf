---
name: project-verify
description: Verify this Rust repository with the smallest sufficient formatting, clippy, check, and test commands before claiming work is complete.
---

# Project Verify

Use this skill before claiming Rust work in this repository is complete.

1. Read `CLAUDE.md` and prefer its exact commands.
2. If only `.claude/`, `CLAUDE.md`, `.mcp.json`, or `.gitignore` changed, validate JSON and file presence instead of running expensive Rust tests.
3. If `Cargo.toml` contains `[workspace]`, prefer:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

4. For a single crate, prefer:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

5. During development, run targeted commands first, such as `cargo test <test_name>` or `cargo test -p <package_name>`.

Only report completion with the exact commands run and their results. If a command was skipped, state why.
