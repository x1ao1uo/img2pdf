#!/usr/bin/env bash
# rust-quality-gate:v3
# Unified strict Rust quality gate for projects under /Volumes/LVLIAN_1T/code.

set -Eeuo pipefail
IFS=$'\n\t'

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$script_dir/Cargo.toml"
cargo_bin="${CARGO:-cargo}"

fail() {
    echo "quality-gate: $*" >&2
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

test -f "$manifest" || fail "Cargo.toml not found next to quality-gate.sh"
require_command "$cargo_bin"

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
has_documentable_target=0
if printf '%s' "$workspace_metadata" |
    grep -Eq '"kind":\[[^]]*"(lib|rlib|dylib|cdylib|staticlib|proc-macro)"'; then
    has_documentable_target=1
fi

printf '===== Rust quality gate =====\n'
printf 'script:    %s\n' "$script_dir/quality-gate.sh"
printf 'workspace: %s\n' "$workspace_root"
printf 'toolchain: %s\n' "$(rustc --version)"

if [[ "${ENABLE_UPGRADE:-0}" == "1" ]]; then
    require_cargo_command upgrade
    run "$cargo_bin" upgrade \
        --incompatible allow \
        --pinned allow \
        --recursive true
    run "$cargo_bin" update
fi

test -f "$workspace_root/Cargo.lock" ||
    fail "Cargo.lock is required for reproducible checks and security audit"
locked=(--locked)

run "$cargo_bin" fmt \
    --manifest-path "$workspace_manifest" \
    --all \
    -- \
    --check

run "$cargo_bin" check \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}"

run "$cargo_bin" clippy \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}" \
    -- \
    -D warnings

run "$cargo_bin" test \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    "${locked[@]}" \
    -- \
    --test-threads=1

if [[ "$has_documentable_target" -eq 1 ]]; then
    run "$cargo_bin" test \
        --manifest-path "$workspace_manifest" \
        --workspace \
        --doc \
        --all-features \
        "${locked[@]}"
else
    printf '\n----- cargo test --doc (skipped: no library target) -----\n'
fi

run "$cargo_bin" build \
    --manifest-path "$workspace_manifest" \
    --workspace \
    --all-targets \
    --all-features \
    --release \
    "${locked[@]}"

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

require_cargo_command audit
printf '\n----- cargo audit -----\n'
if ! "$cargo_bin" audit; then
    echo "cargo audit online refresh failed; retrying with cached advisory database" >&2
    "$cargo_bin" audit --no-fetch
fi

if [[ -f "$workspace_root/deny.toml" ]]; then
    require_cargo_command deny
    run "$cargo_bin" deny check
fi

if [[ "${ENABLE_NIGHTLY:-0}" == "1" ]]; then
    "$cargo_bin" +nightly --version >/dev/null 2>&1 ||
        fail "nightly toolchain is required: rustup toolchain install nightly --component rustfmt"
    run "$cargo_bin" +nightly fmt \
        --manifest-path "$workspace_manifest" \
        --all \
        -- \
        --check
    "$cargo_bin" +nightly udeps --version >/dev/null 2>&1 ||
        fail "cargo-udeps is required: cargo +nightly install cargo-udeps --locked"
    run "$cargo_bin" +nightly udeps \
        --manifest-path "$workspace_manifest" \
        --workspace \
        --all-targets \
        --all-features
fi

printf '\n===== PASSED: %s =====\n' "$workspace_root"
