# Spike 001 · 音频引擎架构验证（B 路线：Rust 内核）

- 日期：2026-11-15 ~ 16
- 时间盒：1 天
- 环境：云端 Linux 工作区，2 核 CPU，无 GPU、无声卡；Chromium 153（无头），Tauri 2.12 + WebKitGTK（Xvfb）
- 代码：`crates/engine-*`、`crates/audio-io`、`apps/desktop`、`experiments/`

## 要回答的问题与结论

| # | 问题 | 结论 | 证据 |
| --- | --- | --- | --- |
| 1 | 同一个 Rust DSP crate 能否同时编译到原生与 wasm32？ | **能**，零条件编译 | `engine-core` 未用任何 `cfg`，两端共用 |
| 2 | 两端输出是否一致？ | **逐采样完全一致** | 81.48 万帧立体声，差异样本 0（原生 vs Node V8 中的 WASM vs Chromium AudioWorklet 中的 WASM） |
| 3 | WASM 在 AudioWorklet 中的 CPU 余量？ | 64 个合成器声部平均占 128 帧预算（2667 µs）的 **2.2%** | 见下表 |
| 4 | 能否在真实浏览器 AudioWorklet 中运行？ | **能**；离线渲染（导出）**124–159× 实时**；实时上下文 376 块/秒（理论 375） | `experiments/browser-worklet` |
| 5 | UI → 音频线程的消息通道 | 浏览器：SharedArrayBuffer 环形缓冲，命令→发声中位 **8.4 ms**、P95 12.8 ms（含 JS 轮询粒度）；原生：rtrb 环形缓冲，6001 条命令 0 丢失、0 迟到块 | `packages/engine-web/src/ring.js`、`chordlab-engine rt-sim` |
| 6 | 调度是否采样级精确？ | **是**；块大小 1/37/128/1024 下首个音符都落在第 24000 帧 | `tests/engine.rs::scheduling_is_sample_accurate` |
| 7 | 音频线程是否零分配？ | **是**；计数分配器验证 `process()` 0 次分配 | `tests/engine.rs::process_is_allocation_free` |
| 8 | Tauri 2 桌面 + 原生音频能否跑通？ | **能**；二进制 5.6 MB；IPC 往返中位 1 ms / P95 4 ms；命令→发声中位 2 ms | `apps/desktop`，`SPIKE_REPORT` |
| 9 | 真实钢琴采样（AKAI Steinway，公有领域）能否加载？ | **能**，但整套解码后 **259 MB**（1469 秒音频，单声道 f32） | `chordlab-engine piano` |
| 10 | 编曲 → 渲染 → 和弦识别闭环 | 引擎渲染的 C–G–Am–F 钢琴音频被 madmom **100%** 识别正确 | Spike 记录 |
| 11 | 工程每次编辑都整体重载可行吗？ | 8 轨 4 千音符：解析 0.3 ms + 编译 0.2 ms ✅；32 轨 6.4 万：约 10 ms ⚠；64 轨 25.6 万：约 53 ms ❌ | `tests/scale.rs` |
| 12 | Rust 模型能否自动生成 TS 类型？ | **能**（ts-rs），`cargo test -p engine-core --features ts` 输出到 `packages/engine-web/src/types/` | — |
| 13 | Python 精确分析服务能否打包给桌面版？ | **能**（PyInstaller），但 **198 MB**（压缩 90 MB），冷启动约 7 s | Spike 记录 |
| 14 | 音频解码（symphonia）能否在 wasm32 编译？ | **能** | `cargo build --target wasm32-unknown-unknown -p audio-io` |

## 性能数据

每个 128 帧音频块（48 kHz，预算 2667 µs）的平均耗时：

| 声部数 | 原生 合成器 | 原生 采样器 | WASM（V8）合成器 |
| --- | --- | --- | --- |
| 8 | 9.7 µs (0.4%) | 9.1 µs (0.3%) | 21.7 µs (0.8%) |
| 16 | 14.8 µs (0.6%) | 12.9 µs (0.5%) | 31.5 µs (1.2%) |
| 32 | 24.1 µs (0.9%) | 21.2 µs (0.8%) | 47.9 µs (1.8%) |
| 64 | 46.1 µs (1.7%) | 32.4 µs (1.2%) | 58.1 µs (2.2%) |

- 原生离线导出：33 秒的 4 轨工程渲染耗时 142 ms（233× 实时）。
- WASM 模块 227 KB（含 serde_json），实例化 5.5 ms。

## 发现的坑（已在代码中处理）

1. **AudioWorkletGlobalScope 没有 `TextEncoder`/`TextDecoder`**：工程 JSON 必须在主线程编码成字节再发给 worklet。这也是不用 wasm-bindgen、改用纯 C ABI 的原因。
2. **`cpal::Stream` 不是 `Send`**：音频流必须由专属线程创建并持有，只把环形缓冲的句柄交给其他线程（见 `engine-native::realtime::Host`）。
3. **SharedArrayBuffer 需要跨源隔离**（COOP/COEP 头）。Web 版部署时必须加这两个响应头；桌面版走原生音频，不需要。
4. **跨平台位级一致**的前提是音频路径上不调用 libm（`sin`/`exp`/`powf`）。`dsp.rs` 只用四则运算和自写多项式近似。
5. **采样器需要力度分层**（`SampleZone.lo_vel/hi_vel`）。真实钢琴有 5 个力度层。
6. 无头 Chromium 的 Lambda 启动参数会关掉站点隔离，导致 `crossOriginIsolated=false`，测试时需去掉这些参数。
7. WebKitGTK 中 `performance.now()` 精度约 1 ms，不能用来在 UI 侧做精确计时（引擎侧计时不受影响）。

## 未能在本环境验证（需要在你的 Mac/PC 上复测）

- 真实声卡下的回调稳定性和输出延迟（容器内的 ALSA 空设备不按真实时间节奏消耗音频，回调数据无效）。
- macOS（WKWebView）与 Windows（WebView2）下的 Tauri 行为。
- Web 版在 Safari 中的表现。

## 对架构的影响

- 采纳：ADR-0001（Rust 内核双目标）、ADR-0003（命令协议与线程模型）、ADR-0004（工程模型与类型生成）。
- 新增约束：采样内存预算（ADR-0005），精确分析服务作为可选下载组件（ADR-0006）。
- 新增待办：大工程增量更新、立体声采样器、采样流式加载（桌面）、BTC/ONNX 推理移植的验证。
