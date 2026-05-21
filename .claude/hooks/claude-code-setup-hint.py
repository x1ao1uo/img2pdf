#!/usr/bin/env python3
import json
import os
import sys

MARKER = "claude-code-setup-verify-hint"
STACK = 'rust'

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

if name in {"settings.json", ".mcp.json"} or path.endswith(".json"):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            json.load(fh)
    except Exception as exc:
        message = f"{MARKER}: JSON validation failed for {path}: {exc}"

if message is None and STACK == "rust" and (ext == ".rs" or name in {"Cargo.toml", "Cargo.lock"}):
    message = f"{MARKER}: Rust metadata/source changed. Before claiming completion, use /project-verify or run the documented cargo fmt/check/clippy/test commands."
elif message is None and STACK == "android" and (ext in {".kt", ".kts", ".java", ".xml"} or name.startswith("build.gradle") or name.startswith("settings.gradle")):
    message = f"{MARKER}: Android/Gradle file changed. Before claiming completion, use /project-verify and the smallest documented Gradle verification task."
elif message is None and STACK == "python" and ext == ".py":
    message = f"{MARKER}: Python file changed. Before claiming completion, use /project-verify or run compileall/pytest as appropriate."
elif message is None and STACK == "tauri" and (ext in {".ts", ".tsx", ".js", ".jsx", ".rs"} or name in {"package.json", "Cargo.toml", "tauri.conf.json"}):
    message = f"{MARKER}: Package/Tauri file changed. Before claiming completion, use /project-verify and the smallest lint/test/check command."
elif message is None and path.endswith(("CLAUDE.md", ".md", ".toml", ".yaml", ".yml")):
    message = f"{MARKER}: Project metadata changed. Before claiming completion, use /project-verify for lightweight validation."

if message:
    print(json.dumps({"systemMessage": message}, ensure_ascii=False))
