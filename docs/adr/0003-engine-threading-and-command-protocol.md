# ADR-0003: 引擎线程模型与命令协议

- 状态：已采纳
- 日期：2026-11-16
- 相关：[Spike 001](../spikes/001-engine-architecture.md) 问题 5、7、8

## 决定

**1. 实时命令：固定 16 字节**（`engine_core::Command::encode` → `[u32; 4]`）。Play / Stop / Seek / NoteOn / NoteOff / SetTrackGain / SetTrackPan / SetMute / SetLoop，后续追加操作码即可。

**2. 传输方式**

| 平台 | UI → 音频线程 | 音频线程 → UI |
| --- | --- | --- |
| 桌面 | Tauri command → `rtrb` 无锁 SPSC 环形缓冲 → cpal 回调 | 原子变量（播放位置、声部数、回调耗时）由 UI 轮询 |
| Web | `RingWriter`（SharedArrayBuffer + Atomics）→ `RingReader`，在每个 `process()` 开头读取 | 状态 SAB（Float64Array）由 UI 在 rAF 中读取 |

命令在下一个音频块边界生效，最大额外延迟 1 块（128 帧 = 2.67 ms）。

**3. 重量级操作**（加载工程、加载采样）不走实时通道：
- 桌面：在普通线程构建完整 `Engine`，经第二条 SPSC 队列整体替换；旧引擎回传到非实时线程释放。
- Web：通过 `port.postMessage` 发送已编码的字节；超大工程的编译需移到 Worker（见 ADR-0004）。

**4. 硬规则**：`Engine::process()` 不分配内存、不加锁、不做 I/O。由 `tests/engine.rs::process_is_allocation_free` 守护。

**5. `cpal::Stream` 由专属线程持有**（它不是 `Send`）。

## 后果

- 前端只需一个 `EngineBridge` 接口，两种实现（Tauri / AudioWorklet），命令编码完全相同。
- 实测：原生 6001 条命令 0 丢失；浏览器命令→发声中位 8.4 ms；桌面 IPC 中位 1 ms。
