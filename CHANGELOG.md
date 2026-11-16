# Changelog

格式参考 [Keep a Changelog](https://keepachangelog.com/)；每个 Sprint 结束发布 `v0.<sprint>.0`。

## [Unreleased] — Sprint 1

## [0.0.0] — 2026-11-16 · Sprint 0

标签 `v0.0.0` 指向 `c5766ef`：除 Sprint 0 的全部内容外，还包含 Sprint 1 的第一个故事 #8。

### Fixed（UX 测试反馈）
- 按住键盘数字键会叠加和弦、松开后卡音（#42）：引擎同音高重触发；界面忽略按键自动重复、失焦时释放。
- delay 回声不在节拍上（#43）：延迟按速度同步，默认八分音符。
- 未完成模块只有标题（#44）：统一标注“即将推出”，首页增加路线图与“首页”导航。
- 停止后 delay 尾音产生非规格化数导致回调变慢（#48）；诊断显示最近 1 秒与本次运行的回调峰值。

### Added
- 采样以 i16 存储（内存减半），支持立体声采样（#8，Sprint 1）。
- 中英双语应用骨架；“引擎诊断”页通过 EngineBridge 驱动桌面原生引擎或浏览器 AudioWorklet 引擎。
- TDD 与每 Sprint 两次 UX 测试流程；三平台预览安装包；浏览器与桌面端到端测试。
- Rust 音频内核 `engine-core`：工程模型、采样级调度、合成器 / 采样器（力度分层）/ 鼓、混音、乒乓延迟发送、离线渲染。
- 原生宿主 `engine-native`：cpal 实时驱动（专属线程 + rtrb 无锁队列 + 引擎热替换）、导出 / 基准 / 实时模拟 CLI。
- WASM 宿主 `engine-wasm`（C ABI）与 AudioWorklet 处理器、SharedArrayBuffer 命令环形缓冲。
- 音频解码 `audio-io`（symphonia + rubato）。
- Tauri 2 桌面壳原型。
- 开源许可：MIT OR Apache-2.0（ADR-0007）、第三方声明、cargo-deny 许可证检查。
- 开发文档、ADR 0001–0007、Spike 000/001、贡献规范、Issue/PR 模板、CI。
