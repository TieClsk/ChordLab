# ADR-0002: 单仓库（monorepo）与工具链

- 状态：已采纳
- 日期：2026-11-16

## 决定

| 目录 | 内容 | 工具 |
| --- | --- | --- |
| `crates/engine-core` | 音频内核（平台无关） | Cargo workspace |
| `crates/engine-native` | 原生宿主（cpal）+ 导出 CLI | Cargo |
| `crates/engine-wasm` | WASM 宿主（C ABI） | Cargo，目标 wasm32 |
| `crates/audio-io` | 音频解码（symphonia）与重采样（rubato） | Cargo |
| `apps/desktop` | Tauri 2 桌面壳 | Cargo + pnpm |
| `apps/web` | Web 前端（React + TypeScript + Vite），桌面与 Web 共用 | pnpm workspace |
| `packages/engine-web` | AudioWorklet 宿主、SAB 环形缓冲、生成的 TS 类型 | pnpm |
| `packages/theory` | 乐理与和弦推荐（tonal 封装） | pnpm |
| `services/analyzer` | Python 精确和弦分析（可选组件） | uv / pip |
| `experiments/` | 技术验证代码，不进入产品 | — |
| `docs/` | 开发文档（见 docs/README.md） | Markdown |

- 前端框架：React + TypeScript + Vite；状态管理 Zustand；国际化 i18next（中英双语，所有用户可见文本走翻译键）。
- 质量：`cargo fmt`、`cargo clippy -D warnings`、`cargo test`；前端 ESLint + Vitest + Playwright。
- CI：GitHub Actions，见 `.github/workflows/ci.yml`。

## 后果

- 一个 PR 可以同时改引擎和界面，保持两端一致。
- 新成员只需装 Rust（含 wasm32 目标）、Node 22 + pnpm、可选 Python。
