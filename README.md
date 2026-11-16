# ChordLab

看得见、听得到、能拆解的和弦学习与创作工具。
*See it, hear it, take it apart — a chord learning and songwriting workstation.*

> 状态 / Status：**Sprint 0 — 基础架构 / foundations**。还没有可用的产品界面 / No user-facing app yet.

## 功能规划 / Planned features

| 史诗 / Epic | 内容 / Scope |
| --- | --- |
| E1 互动钢琴 Interactive piano | 弹奏、和弦高亮、下一个和弦推荐、进行循环 |
| E2 和弦教程 Lessons | 和弦原理、常见进行互动演示、流行歌曲示例 |
| E3 音乐分析 Analysis | 批量上传、调/BPM/和弦识别、规律统计、记录管理 |
| E4 音色 Sounds | 钢琴与其他音色 |
| E5 编曲 Arrangement | 钢琴卷帘、鼓、合成器、采样、混音、导出 |

## 架构 / Architecture

一份 React 界面 + 一个 Rust 音频内核；内核同时编译为原生（桌面版，Tauri 2 + cpal）和 WebAssembly（Web 版，AudioWorklet），两端输出逐采样一致。
详见 [docs/architecture/overview.md](docs/architecture/overview.md)。

```
crates/engine-core     音频内核 (Rust, platform-agnostic)
crates/engine-native   桌面音频宿主 + 导出 CLI
crates/engine-wasm     WASM 宿主 (C ABI for AudioWorklet)
crates/audio-io        音频解码 / 重采样
apps/desktop           Tauri 2 桌面壳
apps/web               前端 (React + TS)            [S0 进行中]
packages/engine-web    AudioWorklet 处理器、SAB 环形缓冲、生成的 TS 类型
packages/theory        乐理与和弦推荐                 [S1]
services/analyzer      Python 精确和弦识别（可选组件）
experiments/           技术验证代码
docs/                  开发文档
```

## 开发 / Development

- 环境搭建 Setup：[docs/dev/setup.md](docs/dev/setup.md)
- 流程与分支 Process：[CONTRIBUTING.md](CONTRIBUTING.md)
- 文档索引 Docs index：[docs/README.md](docs/README.md)
