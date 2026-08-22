#!/usr/bin/env bash
# rust-quality-gate:v4（通用母版）
# Unified strict Rust quality gate for projects under /Volumes/LVLIAN_1T/code.
#
# 用法：把本文件拷到任意 Rust 项目根（workspace 根），chmod +x 后 ./quality-gate.sh。
# 已配置实例：shudong/quality-gate.sh（COV_MIN=91）、token-balance/quality-gate.sh（COV_MIN=100）。
#
# ── 项目级配置（环境变量同名可覆盖）──────────────────────────────
# 行覆盖率阈值（%）。默认空 = 跳过覆盖率门禁；
# 接入项目时先跑一次 cargo llvm-cov 测基线，把阈值定在基线值，后续只升不降。
COV_MIN="${COV_MIN:-}"
# 不计入覆盖率的文件正则（如 '(src/main\.rs|src/lib\.rs)$' 排除入口胶水层）。
COV_EXCLUDE="${COV_EXCLUDE:-}"
# 1 = 先 cargo fmt 自动修再 --check（本地）；0 = 纯 --check（CI）。
FMT_FIX="${FMT_FIX:-1}"
# ────────────────────────────────────────────────────────────────
#
# 依赖（需预先安装，全部为 stable 工具）：
#   cargo-audit cargo-llvm-cov cargo-nextest cargo-machete cargo-outdated
#   cargo-deny（存在 deny.toml 时必需）
# 可选重检查：MUTANTS=1 时跑 cargo-mutants（耗时长，适合发布前/夜间）。
# 不引入：cargo +nightly udeps（违反 stable 规则；用 cargo-machete 替代）。
# 不引入：cargo tarpaulin（macOS 不支持 ptrace，用跨平台 cargo-llvm-cov）。
# 不引入：cargo-geiger（半停维护；改用 Cargo.toml 的 [lints.rust] unsafe_code = "deny"）。

set -Eeuo pipefail
IFS=$'\n\t'

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$script_dir/Cargo.toml"
cargo_bin="${CARGO:-cargo}"

fail() {
    echo "gates: $*" >&2
    exit 1
}

run() {
    printf '\n-----'
    printf ' %q' "$@"
    printf ' -----\n'
    "$@"
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

require_cargo_command() {
    "$cargo_bin" "$1" --version >/dev/null 2>&1 ||
        fail "missing cargo-$1; install with: cargo install cargo-$1 --locked"
}

test -f "$manifest" || fail "Cargo.toml not found next to gates.sh"
require_command "$cargo_bin"
require_command rustc

rustc_release="$(rustc -vV | sed -n 's/^release: //p')"
[[ "$rustc_release" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "stable Rust is required; found release: ${rustc_release:-unknown}"

workspace_manifest="$(
    "$cargo_bin" locate-project \
        --manifest-path "$manifest" \
        --workspace \
        --message-format plain
)"
workspace_root="$(cd "$(dirname "$workspace_manifest")" && pwd)"
cd "$workspace_root"

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-short}"

workspace_metadata="$(
    "$cargo_bin" metadata \
        --manifest-path "$workspace_manifest" \
        --no-deps \
        --format-version 1
)"
# cargo doc 支持所有 lib 类 target；但 cargo test --doc 只认 lib/rlib/proc-macro，
# cdylib/staticlib/dylib 单独存在时不支持 doc test（如 JNI/FFI crate），需区分。
has_documentable_target=0
if printf '%s' "$workspace_metadata" |
    grep -Eq '"kind":\[[^]]*"(lib|rlib|dylib|cdylib|staticlib|proc-macro)"'; then
    has_documentable_target=1
fi
has_doc_test_target=0
if printf '%s' "$workspace_metadata" |
    grep -Eq '"kind":\[[^]]*"(lib|rlib|proc-macro)"'; then
    has_doc_test_target=1
fi

printf '===== Rust quality gate v4 =====\n'
printf 'script:    %s\n' "${BASH_SOURCE[0]}"
printf 'workspace: %s\n' "$workspace_root"
printf 'toolchain: %s\n' "$(rustc --version)"

# UPGRADE 与历史文档里的 ENABLE_UPGRADE 两种写法都认
if [[ "${UPGRADE:-${ENABLE_UPGRADE:-0}}" == "1" ]]; then
    require_cargo_command upgrade
    require_cargo_command outdated
    run "$cargo_bin" outdated --root-deps-only
    run "$cargo_bin" upgrade \
        --compatible allow \
        --incompatible allow \
        --pinned allow \
        --recursive true
    run "$cargo_bin" update
else
    printf '\n----- cargo upgrade + update (skipped: set UPGRADE=1 to enable) -----\n'
fi

if [[ "${CARGO_CLEAN:-0}" == "1" ]]; then
    run "$cargo_bin" clean --manifest-path "$workspace_manifest"
fi

test -f "$workspace_root/Cargo.lock" ||
    fail "Cargo.lock is required for reproducible checks and security audit"
locked=(--locked)

# 1. 代码格式：默认先自动修再校验（本地友好）；FMT_FIX=0 时纯 --check（CI）
if [[ "$FMT_FIX" == "1" ]]; then
    run "$cargo_bin" fmt --manifest-path "$workspace_manifest" --all
fi
run "$cargo_bin" fmt \
    --manifest-path "$workspace_manifest" \
    --all \
    -- \
    --check

# 2. 编译检查
run "$cargo_bin" check \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}"

# 3. 静态检查：默认 features + all-features 各一遍
run "$cargo_bin" clippy \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    "${locked[@]}" \
    -- \
    -D warnings

run "$cargo_bin" clippy \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}" \
    -- \
    -D warnings

# 4. 测试：nextest（进程级隔离，防测试间共享资源互相污染），
#    默认 features + all-features 各一遍
require_cargo_command nextest
run "$cargo_bin" nextest run \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    "${locked[@]}"

run "$cargo_bin" nextest run \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}"

# 5. 文档测试（有 lib/rlib/proc-macro target 时；纯 cdylib/staticlib 不支持）
if [[ "$has_doc_test_target" -eq 1 ]]; then
    run "$cargo_bin" test \
        --manifest-path "$workspace_manifest" \
        --workspace \
        --doc \
        --all-features \
        "${locked[@]}"
else
    printf '\n----- cargo test --doc (skipped: no doc-testable library target) -----\n'
fi

# 6. 行覆盖率门禁（复用 nextest runner）
if [[ -n "$COV_MIN" ]]; then
    require_cargo_command llvm-cov
    cov_args=(
        nextest
        --workspace
        --all-features
        --locked
        --fail-under-lines "$COV_MIN"
    )
    if [[ -n "$COV_EXCLUDE" ]]; then
        cov_args+=(--ignore-filename-regex "$COV_EXCLUDE")
    fi
    run "$cargo_bin" llvm-cov "${cov_args[@]}"
else
    printf '\n----- cargo llvm-cov (skipped: COV_MIN is empty) -----\n'
fi

# 7. release 构建
run "$cargo_bin" build \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    --release \
    "${locked[@]}"

# 8. rustdoc（-D warnings，有 lib target 时）
if [[ "$has_documentable_target" -eq 1 ]]; then
    printf '\n----- rustdoc (-D warnings) -----\n'
    RUSTDOCFLAGS="${RUSTDOCFLAGS:+$RUSTDOCFLAGS }-D warnings" \
        "$cargo_bin" doc \
            --manifest-path "$workspace_manifest" \
            --workspace \
            --all-features \
            --no-deps \
            "${locked[@]}"
else
    printf '\n----- rustdoc (skipped: no library target) -----\n'
fi

# 9. 项目自定义检查钩子
project_gate="$workspace_root/scripts/quality-gate-project.sh"
if [[ -f "$project_gate" ]]; then
    run bash "$project_gate"
fi

# 10. 未使用依赖（stable 替代 cargo-udeps）
require_cargo_command machete
run "$cargo_bin" machete

# 11. 依赖漏洞审计（网络失败时用本地缓存的 advisory 库兜底重试）
require_cargo_command audit
printf '\n----- cargo audit -----\n'
if ! "$cargo_bin" audit; then
    echo "cargo audit online refresh failed; retrying with cached advisory database" >&2
    "$cargo_bin" audit --no-fetch || fail "cargo audit found vulnerabilities or warnings (see above)"
fi

# 12. 依赖策略检查（存在 deny.toml 时；网络失败时用本地缓存的 advisory 库兜底重试）
if [[ -f "$workspace_root/deny.toml" ]]; then
    require_cargo_command deny
    printf '\n----- cargo deny check -----\n'
    if ! "$cargo_bin" deny check; then
        echo "cargo deny online fetch failed; retrying with cached advisory database" >&2
        "$cargo_bin" deny --disable-fetch check ||
            fail "cargo deny found violations (see above)"
    fi
fi

# 13. 变异测试（验证测试套件本身的有效性；耗时长，默认跳过，MUTANTS=1 开启）
if [[ "${MUTANTS:-0}" == "1" ]]; then
    require_cargo_command mutants
    run "$cargo_bin" mutants --manifest-path "$workspace_manifest"
else
    printf '\n----- cargo mutants (skipped: set MUTANTS=1 to enable) -----\n'
fi

printf '\n===== PASSED: %s =====\n' "$workspace_root"
