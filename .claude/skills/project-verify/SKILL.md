---
name: project-verify
description: Verify this repository with the smallest sufficient formatting, linting, build, and test commands before claiming work is complete.
---

# Project Verify

Use this skill before claiming work in this repository is complete.

## Required verification flow

1. Read `CLAUDE.md` and prefer its exact commands first.
2. If only `.claude/`, `CLAUDE.md`, `.mcp.json`, or `.gitignore` changed, validate JSON and file presence instead of running expensive build/test commands.
3. Pick the appropriate language commands from below if `CLAUDE.md` does not give a smaller subset.

## Language commands

### Rust (single crate)
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

### Rust (workspace)
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Android / Gradle
```bash
./gradlew spotlessCheck || ./gradlew lint<Variant>
./gradlew :<module>:assemble<Variant>
```

### Go
```bash
gofmt -l .
go vet ./...
go test ./...
```

### Python
```bash
ruff check .
pytest
```

### Node / TypeScript
```bash
pnpm lint
pnpm typecheck
pnpm test
```

### Shell
```bash
shellcheck <file>
```

## Targeted tests

During development, run the smallest targeted command that covers the changed area before the full gate. Examples:
- `cargo test <test_name>`
- `cargo test -p <package_name>`
- `./gradlew :<module>:test<Variant>UnitTest`
- `go test ./pkg/foo/...`

## Completion rule

Only claim completion after reporting the exact commands run and whether they passed. If a command was skipped, state why.
