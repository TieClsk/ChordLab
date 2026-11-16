# 本地开发环境 / Development setup

## 必需

| 工具 | 版本 | 说明 |
| --- | --- | --- |
| Rust（rustup） | stable ≥ 1.85 | `rustup target add wasm32-unknown-unknown` |
| Node.js | 22 LTS | 自带 corepack；`corepack enable pnpm` |
| 系统依赖（Linux） | — | `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libasound2-dev libssl-dev pkg-config build-essential` |
| 系统依赖（macOS） | — | Xcode Command Line Tools |
| 系统依赖（Windows） | — | Visual Studio Build Tools（C++）、WebView2（Win11 自带） |

## 可选

- Python 3.11+：仅开发 `services/analyzer` 或跑识别评测时需要。

## 常用命令

```bash
# 引擎测试（含零分配、采样级调度、一致性等）
cargo test --workspace --exclude chordlab-desktop

# 引擎性能与实时模拟
cargo run --release -p engine-native --bin chordlab-engine -- bench
cargo run --release -p engine-native --bin chordlab-engine -- rt-sim

# 导出演示工程为 WAV
cargo run --release -p engine-native --bin chordlab-engine -- render demo demo.wav 16

# WASM 构建 + 与原生逐采样比对
cargo build --release -p engine-wasm --target wasm32-unknown-unknown
cargo run --release -p engine-native --bin chordlab-engine -- raw native.f32
node experiments/wasm-parity/run.mjs target/wasm32-unknown-unknown/release/engine_wasm.wasm native.f32

# 前端（apps/web）
pnpm install
pnpm --filter @chordlab/web dev      # 开发服务器（已带 COOP/COEP 头）
pnpm test && pnpm lint && pnpm typecheck

# 重新生成前端 TS 类型（模型改动后）
cargo test -p engine-core --features ts --lib

# 桌面应用（开发）
cargo run -p chordlab-desktop
```

## 目录

见 [ADR-0002](../adr/0002-monorepo-and-tooling.md) 和 [架构总览](../architecture/overview.md)。
