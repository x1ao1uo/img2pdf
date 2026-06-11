# Project Agent Guide

本文件是本仓库的项目级工作规范（与 AGENTS.md / Claude Code CLAUDE.md 协议对齐）。
- 项目根目录下生效；子目录可被 `AGENTS.override.md` 覆盖。
- 若仓库同时存在 `AGENTS.md`，内容应与本文件保持一致；更新其中一份必须同步另一份。

## 1) 协作与交付默认值
- 默认中文沟通；除非用户明确要求英文。
- 目标优先：给出可直接执行的步骤，不讲空泛方案。
- 不能将历史会话状态当作当前事实；涉及结论请基于当前命令/文件/工具输出。
- 不擅自解释无法确认的技术细节；缺证据时明确标注待核实项与风险。
- 不删除、回滚、覆盖用户既有改动，除非用户明确要求。
- 先做最小闭环：优先实现最小可行改动，再扩展。
- 优化、重构、清理类任务默认先做只读盘点，避免一次性大规模变更。
- 每次结尾说明：已完成、未完成验证、下一步建议（如有）。

## 2) 任务执行流程（最小闭环）
1. 先判断本次任务范围（构建/架构/CLI/库/脚本…）。
2. 只读盘点相关文件与调用链。
3. 明确"只改哪些文件 + 风险等级 + 回退点"。
4. 执行最小改动并补关键验证。
5. 给出结论、未完成项、下一步建议。

## 3) 提交粒度与分支流程
- 所有代码改动先在临时分支完成。
- 验证通过后再合并回主分支并删除临时分支。
- 每完成一个独立可编译/可验证语义单元立即 commit。
- 每个 commit 需可独立编译/运行；尽量可跑通 lint/测试。
- 不要把无关改动混入同一个 commit。
- commit 信息中文，subject ≤ 50 字，body 说明"为什么"。
- 每次 commit 后输出 7 位短 SHA，便于回溯与回滚。
- 子任务结束前尽量确认工作树与 commit 可追溯性（如 `git status`、分支 ref）。

## 4) 验证与门禁
- 变更完成后按任务范围执行最小验证链（编译、单测、lint、关键模块 build）。
- `cargo clippy` 必须通过才能算"完成"。
- 无法验证时必须明确说明原因与下一步；不要以"跑过了"作为完成证据。

## 5) 代码结构分析优先链
- 涉及调用链、依赖图、影响面、符号定位时，优先使用 `codegraph_*` 工具链。
- 不要"代用"不存在能力；无调用则明确说明。

## 6) GitHub 操作规范
- 优先使用 GitHub 官方 CLI（`gh`）。
- `gh` 已通过本机 keyring 认证（`gh auth status` 可核验），默认不需要 PAT。
- 常用命令映射：`gh pr view` / `gh issue view` / `gh repo view` / `gh pr create` / `gh pr merge` / `gh pr review` / `gh api`。

## 7) Rust 通用约定
- **Edition**：`edition = "2021"` 或 `"2024"`。
- **异步运行时**：`tokio`，开 `features = ["full"]`。
- **错误处理**：应用层用 `anyhow`，库层用 `thiserror` 定义错误类型。
- **日志**：`tracing` + `tracing-subscriber`（`env-filter` 特性）。
- **Windows GUI**：`main.rs` 顶部放 `#![windows_subsystem = "windows"]` 抑制终端。
- **路径**：Windows 优先用正斜杠（`/`），不要写死反斜杠。
- **目标平台**：默认 Windows；跨平台时用 `cfg` 显式标注。

## 8) Rust 提交流程
- 改动后必跑：
  - `cargo fmt --check`（或 `cargo fmt` 自动修复）
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo check`
  - 按需：`cargo test` / `cargo build --release`
- 涉及 Android NDK：`cargo ndk -t arm64-v8a --platform 24 -o target/ndk-build build --release`。

## 9) Rust 常见依赖栈
| 用途 | 选型 |
|---|---|
| Web 框架 | `axum` / `salvo` / `warp` |
| ORM | `sea-orm`（异步）/ `rbatis`（宏） |
| HTTP 客户端 | `reqwest`（`json` / `cookies` / `form`） |
| 序列化 | `serde` + `serde_json` |
| 配置 | `toml` via `config` crate，或 `dotenvy` for `.env` |
| 校验 | `validator`（`derive` 特性） |
| 加密 | `aes` / `md5` / `sha2` / `hex` / `base64`（直接依赖） |

## 10) Rust 共享工具库（按需引用）
根目录的共享 crate（位于 `/Volumes/LVLIAN_1T/code/`）：

| Crate | 用途 |
|---|---|
| `z1nt_tools` | 共享小工具（加密、编码、FFI、time） |
| `sql_sqlx` | SQLx MySQL 池（设时区 `+08:00`） |
| `sql_rbatis_ext` | 共享 RBatis MySQL 池工厂（`get_rbatis_pool`） |
| `web_common` | 共享 web 层类型/工具 |

引用方式（`Cargo.toml`）：
```toml
[dependencies]
z1nt_tools = { path = "../z1nt_tools" }
sql_rbatis_ext = { path = "../sql_rbatis_ext" }
```
