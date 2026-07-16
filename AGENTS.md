# Rust Project Agent Rules

<!-- rust-quality-gate-policy:v3 -->
## Rust 质量门禁

本目录中的代码、依赖、配置或构建脚本发生任何变更后，提交或交付前必须运行并通过：

```bash
./quality-gate.sh
```

需要把所有直接依赖、固定版本依赖和传递依赖升级到 Cargo 当前可解析的最新版时，运行：

```bash
ENABLE_UPGRADE=1 ./quality-gate.sh
```

需要增加 nightly `rustfmt` 和未使用依赖检查时，运行：

```bash
ENABLE_NIGHTLY=1 ./quality-gate.sh
```

任何检查失败都必须修复；不得跳过、忽略失败，或在质量门禁未通过时声称任务完成。
