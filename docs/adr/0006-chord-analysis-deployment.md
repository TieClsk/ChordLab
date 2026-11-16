# ADR-0006: 和弦识别的部署方式

- 状态：已采纳
- 日期：2026-11-16
- 相关：规划文档第 4 节（识别对比）、[Spike 001](../spikes/001-engine-architecture.md) 问题 10、13

## 背景

- 识别准确率最高的是 madmom CNN+CRF（合成测试集 96.1%）；项目为个人非商用，其 CC BY-NC-SA 模型许可可以接受。
- madmom 是 Python；打包成独立程序后 198 MB（压缩 90 MB），冷启动约 7 秒。
- essentia.js 可在浏览器内运行，准确率较低（91.4%），速度快。

## 决定

1. **快速分析**（默认，桌面与 Web 都有）：essentia.js 在 Web Worker 中运行，几秒出结果。
2. **精确分析**（桌面，可选组件）：`services/analyzer`（madmom）打包成 sidecar，用户首次使用时下载，以常驻进程运行（避免每首歌 7 秒冷启动），通过本地 HTTP/stdio 通信。
3. 两种结果都存成同一 `ChordEvent` 结构，支持手动修正。
4. **待验证**：把 BTC（MIT 许可，PyTorch）导出为 ONNX，用 Rust 推理（ort/tract）直接嵌入桌面应用，彻底去掉 Python。安排在 Sprint 4 的 Spike。

## 后果

- 安装包保持小（约 6 MB + 前端资源），重量组件按需下载。
- 需要维护一个 Python 子项目的打包流水线（PyInstaller，三个平台分别构建）。
