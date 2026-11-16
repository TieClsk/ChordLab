# Changelog

格式参考 [Keep a Changelog](https://keepachangelog.com/)；每个 Sprint 结束发布 `v0.<sprint>.0`。

## [Unreleased] — Sprint 0

### Added
- Rust 音频内核 `engine-core`：工程模型、采样级调度、合成器 / 采样器（力度分层）/ 鼓、混音、乒乓延迟发送、离线渲染。
- 原生宿主 `engine-native`：cpal 实时驱动（专属线程 + rtrb 无锁队列 + 引擎热替换）、导出 / 基准 / 实时模拟 CLI。
- WASM 宿主 `engine-wasm`（C ABI）与 AudioWorklet 处理器、SharedArrayBuffer 命令环形缓冲。
- 音频解码 `audio-io`（symphonia + rubato）。
- Tauri 2 桌面壳原型。
- 开源许可：MIT OR Apache-2.0（ADR-0007）、第三方声明、cargo-deny 许可证检查。
- 开发文档、ADR 0001–0007、Spike 000/001、贡献规范、Issue/PR 模板、CI。
