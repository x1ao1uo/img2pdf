## Code Wiki

本项目的架构、模块、依赖、构建/运行命令等综合说明见 [`Code-Wiki.md`](./Code-Wiki.md)(自动维护于 2026-09-08,与源码同步)。修改本仓库前请先阅读 Code-Wiki.md 了解模块边界与约定;Code-Wiki 与源码不一致时,以源码为准并同步更新 Code-Wiki。

# Rust Project Agent Rules

---

## 共享开发规则(自包含,内联自父目录快照 2026-09-08)

> 适用于本目录下全部 Android/Rust 项目。项目级 `AGENTS.md` 可覆盖本文件,项目文件只记项目特有信息;个人全局规则见 `~/.codex/AGENTS.md`。

## 环境事实(2026-08-28 实测,有变化时更新此节)

- Android SDK:`~/Library/Android/sdk`;**platforms 仅保留 android-37.0**(`compileSdk = 37` 即解析到它);NDK 29.0.14206865
- 新版 Android CLI 已装好且在 PATH:命令 `android`(老 `sdkmanager` 已弃用,其弃用警告属正常,无需处理)
- adb 不在 PATH:用全路径 `~/Library/Android/sdk/platform-tools/adb`,或把 `platform-tools` 加入 PATH
- JDK 26(Homebrew)、Rust stable 1.98(edition 2024)、Maven 3.9.16、Gradle 用户级缓存 `~/.gradle`(当前 wrapper 9.7.x)

## 日常高频:android CLI

| 命令 | 用途 |
|---|---|
| `android info` | SDK 位置 + 已连设备 + 环境,开工第一步 |
| `android run` | 构建 + 部署 + 启动一条龙 |
| `android install` | 装 APK(增量优化,不启动) |
| `android screen` | 设备截图 |
| `android layout` | 打印应用 UI tree |
| `android sdk list / install / remove / update` | SDK 包管理(替代 sdkmanager) |
| `android emulator list / start / stop` | AVD 管理 |

## 构建 / 测试(gradlew)

任务名与变体以各项目 `build.gradle.kts` / `libs.versions.toml` 为准,**不发明任务名**:

```bash
./gradlew assembleDebug        # 有变体的项目按实际变体名,如 assembleZ1ntDebug
./gradlew testDebugUnitTest    # 单元测试(变体对应测试任务,如 testProdDebugUnitTest)
./gradlew installDebug
./gradlew lint
./gradlew clean / --stop       # 清理构建 / 杀 daemon
```

## adb(操作前必跑 `adb devices -l`,serial 每次确认)

```bash
adb devices -l
adb -s <serial> logcat --pid=$(adb -s <serial> shell pidof -s <包名>)
adb -s <serial> install -r <apk>
adb -s <serial> shell am start -n <包名>/<Activity>
adb -s <serial> shell pm clear <包名>
```

## 验证要求

UI / 设备相关改动:构建 → 安装 → 启动关键流程 → 截图 / UI tree → logcat;不以编译通过代替运行验证。

## 通用规则(全部仓库)

- **密钥与凭据**:禁止提交密钥、Token、keystore、`local.properties`、`.env`、设备标识;密钥不进源码、日志、提交信息、PR 描述。`.env.example` 只放非机密占位值。
- **Git 边界**:不主动 `git commit` / `push` / `tag` / `merge` / 开 PR,除非用户明确要求;禁止 `git reset --hard`;不在默认分支 force-push。
- **提交规范**:Conventional Commits(`type(scope): summary`,破坏性变更加 `!`);有 `CHANGELOG.md` 的仓库同步维护 `[Unreleased]` 段。
- **制品不入库**:`target/`、`dist/`、`build/`、`logs/`、`.exe` 等生成物不提交;提交前必看 `git status`。

## Rust 项目公共规则

- **质量门禁**:代码、依赖、配置或构建脚本发生任何变更后,提交或交付前必须运行并通过 `./quality-gate.sh`;升级全部依赖到最新可解析版本用 `ENABLE_UPGRADE=1 ./quality-gate.sh`。任何检查失败都必须修复;不得跳过、忽略失败,或在门禁未通过时声称任务完成。
- **标准验证命令**:`cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-features`。
- **工具链与依赖**:只用 stable Rust,禁止 nightly/beta;依赖使用 Cargo 可解析的最新 stable 兼容版本,禁止 alpha/beta/RC、通配版本和无理由旧版;不新增依赖,除非标准库和现有依赖无法覆盖。
- **严格 lint**:不用 `#[allow(...)]` 压警告,改代码;公共 API 错误走类型化错误枚举(thiserror),不让 `anyhow`/裸字符串穿透 API 边界。
- **HTTP 约定**:`reqwest` 必须 `default-features = false`(rustls,不引入 OpenSSL);GET 可退避重试,POST 默认不重试。
- **破坏性测试**:打真实服务/数据库的测试显式 opt-in(默认 `#[ignore]`),不进默认测试矩阵。

## Android 项目公共规则

- **版本唯一来源**:第三方依赖与版本统一放 `gradle/libs.versions.toml`,不在模块 `build.gradle.kts` 里新写版本常量。
- **版权头**:新增源码文件必须带 `spotless/` 模板的版权头,`spotlessCheck` 强制。
- **本地库消费方式**:本地库经 `includeBuild`(composite build)被兄弟仓库消费;不发布到任何远程仓库,不加远程 publishing 配置。
- **架构参考**:尽量模仿官方示例 Now in Android,本地只读克隆在 `/Volumes/LVLIAN_1T/github/nowinandroid`。
