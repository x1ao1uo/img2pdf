#!/usr/bin/env bash
# Rust project quality gate — 顶级 CI 标准
# 与 Cargo.toml 同级放置；运行：./quality-gate.sh
#
# 总是跑且 hard fail（任何一项 fail 即全停）:
#  1. cargo fmt --check
#  2. cargo clippy --all-targets --all-features -- -D warnings
#  3. cargo test --all-features --quiet -- --test-threads=1
#  4. cargo test --doc --all-features
#  5. cargo build --all-targets --all-features --release
#  6. cargo doc --no-deps --all-features
#
# 卫生检查（缺工具就 skip）:
#  7. cargo audit
#  8. cargo udeps --all-targets
#
# 升级门（默认关，环境变量 opt-in）:
#  * ENABLE_UPGRADE=1   → cargo upgrade --incompatible
#  * ENABLE_NIGHTLY=1   → cargo +nightly fmt --check 与 cargo +nightly udeps
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

printf '\n===== Quality gate: %s =====\n' "$(pwd)"

run() { printf '\n----- %s -----\n' "$*"; "$@"; }

if [ "${ENABLE_UPGRADE:-0}" = "1" ]; then
  run cargo upgrade --incompatible || echo "(warn: upgrade 失败; 继续)"
fi

run cargo fmt --check

if [ "${ENABLE_NIGHTLY:-0}" = "1" ] && cargo +nightly --version >/dev/null 2>&1; then
  run cargo +nightly fmt --check
fi

run cargo clippy --all-targets --all-features -- -D warnings
run cargo test --all-features --quiet -- --test-threads=1
run cargo test --doc --all-features
run cargo build --all-targets --all-features --release
run cargo doc --no-deps --all-features

echo "----- cargo audit -----"
if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit
else
  echo "(skip: cargo-audit 未装; 装: cargo install cargo-audit --locked)"
fi

echo "----- cargo udeps -----"
if command -v cargo-udeps >/dev/null 2>&1; then
  if [ "${ENABLE_NIGHTLY:-0}" = "1" ] && cargo +nightly --version >/dev/null 2>&1; then
    cargo +nightly udeps --all-targets
  else
    cargo udeps --all-targets
  fi
else
  echo "(skip: cargo-udeps 未装; 装: cargo install cargo-udeps --locked)"
fi

printf '\n===== ✓ PASSED: %s =====\n' "$(pwd)"
