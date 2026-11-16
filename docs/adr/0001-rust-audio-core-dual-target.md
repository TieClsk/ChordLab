# ADR-0001: 音频引擎用 Rust 编写，同时编译到原生与 WebAssembly

- 状态：已采纳
- 日期：2026-11-16
- 相关：[Spike 001](../spikes/001-engine-architecture.md)

## 背景

产品后期要做编曲（MIDI 编写、鼓、合成器、采样、混音、导出）。现成的 smplr / Tone.js 适合播放和演示，但撑不起编曲软件的内核。第一版主打电脑端，同时希望保留 Web 版。

## 决定

1. 一个与平台无关的 Rust crate `engine-core` 负责：工程模型、采样级事件调度、声部池、乐器（合成器/采样器/鼓）、混音、效果、离线渲染。
2. 两个宿主：
   - `engine-native`：桌面版，由 cpal 驱动音频回调，嵌在 Tauri 2 应用里。
   - `engine-wasm`：Web 版，编译为 wasm32，以纯 C ABI 导出，在 AudioWorkletProcessor 中运行。
3. 音频路径只用确定性运算（不调用 libm），保证两端输出逐采样一致，可用“原生渲染结果”做 Web 端的回归基准。
4. smplr / Tone.js 不进入编曲路径；早期教程和钢琴页也直接用自研引擎，避免两套发声系统。

## 备选方案

| 方案 | 不选的原因 |
| --- | --- |
| A：全 TypeScript 引擎 + Electron | 性能上限受限；无法将来加载原生插件；Electron 体积大 |
| Rust 引擎只做桌面，Web 用 Tone.js | 两套发声逻辑，音色和行为不一致 |
| wasm-bindgen 生成胶水代码 | AudioWorklet 中缺少 TextDecoder 等 API，胶水代码不可用 |

## 后果

- ✅ 实测：64 声部仅占 AudioWorklet 预算 2.2%；原生 1.7%。
- ✅ 一份 DSP 代码，两端结果完全一致。
- ⚠ 需要 Rust 能力；构建需要 `wasm32-unknown-unknown` 目标。
- ⚠ Web 版需要跨源隔离头（COOP/COEP）才能用 SharedArrayBuffer。
- ➡ 后续：立体声采样器、效果器链、自动化、MIDI 输入均在 `engine-core` 中实现。
