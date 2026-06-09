#!/usr/bin/env python3
"""Claude Code 提交流程提示钩子（语言自动检测版）。"""
import json
import os
import sys

MARKER = "claude-code-setup-verify-hint"

try:
    payload = json.load(sys.stdin)
except Exception:
    sys.exit(0)

tool_input = payload.get("tool_input") or {}
tool_response = payload.get("tool_response") or {}
path = tool_input.get("file_path") or tool_response.get("filePath") or ""
if not path:
    sys.exit(0)

name = os.path.basename(path)
ext = os.path.splitext(name)[1].lower()
message = None

# 校验 JSON 文件
if name in {"settings.json", ".mcp.json"} or path.endswith(".json"):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            json.load(fh)
    except Exception as exc:
        message = f"{MARKER}: JSON 校验失败 {path}: {exc}"

# Rust
elif ext == ".rs" or name in {"Cargo.toml", "Cargo.lock"}:
    message = f"{MARKER}: Rust 文件变更。请先跑 /project-verify 或 `cargo fmt` + `cargo clippy` + `cargo test` 再声称完成。"

# Android
elif ext in {".kt", ".kts", ".java", ".xml"} or name.startswith("build.gradle") or name.startswith("settings.gradle"):
    message = f"{MARKER}: Android/Gradle 文件变更。请先跑 /project-verify 或最小 `./gradlew` 验证任务再声称完成。"

# Go
elif ext == ".go" or name == "go.mod" or name == "go.sum":
    message = f"{MARKER}: Go 文件变更。请先跑 /project-verify 或 `gofmt` + `go vet` + `go test` 再声称完成。"

# Python
elif ext == ".py" or name in {"pyproject.toml", "requirements.txt"}:
    message = f"{MARKER}: Python 文件变更。请先跑 /project-verify 或 `ruff check` + `pytest` 再声称完成。"

# TypeScript / JavaScript
elif ext in {".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"} or name == "package.json":
    message = f"{MARKER}: Node/TS 文件变更。请先跑 /project-verify 或 `pnpm lint` + `pnpm typecheck` + `pnpm test` 再声称完成。"

# Shell / PowerShell
elif ext in {".sh", ".bash", ".ps1"}:
    message = f"{MARKER}: 脚本文件变更。请先 `shellcheck` / 手动验证一次再声称完成。"

# Markdown / 文档
elif ext in {".md", ".markdown"} or name in {"CLAUDE.md", "AGENTS.md", "README.md"}:
    message = f"{MARKER}: 文档变更。请用 `git diff --check` 确认无尾随空白再声称完成。"

# 配置文件
elif ext in {".toml", ".yaml", ".yml"} or name in {".gitignore", ".dockerignore"}:
    message = f"{MARKER}: 配置文件变更。请用 `git diff --check` + 解析器校验再声称完成。"

if message:
    print(json.dumps({"systemMessage": message}, ensure_ascii=False))
