# ADR-0007: 开源许可证

- 状态：已采纳
- 日期：2026-11-16

## 背景

仓库公开（public）；项目个人、非商用。部分候选组件许可较严：

| 组件 | 许可 | 在哪里用 |
| --- | --- | --- |
| madmom 预训练模型 | CC BY-NC-SA 4.0（非商用） | 可选下载的精确分析组件 |
| Chordonomicon 数据集 | CC BY-NC 4.0（非商用） | 离线统计和弦转移概率 |
| essentia / essentia.js | AGPL-3.0 | 仅用于 Spike 对比 |
| symphonia 等 19 个依赖 | MPL-2.0（文件级弱 copyleft） | 音频解码 |

## 决定

1. **本项目自有代码采用 `MIT OR Apache-2.0` 双许可**（Rust 生态惯例）。使用者任选其一：MIT 最简单；Apache-2.0 额外提供专利授权。
2. **贡献即按同一双许可授权**（inbound = outbound），写入 CONTRIBUTING。
3. **主程序不链接 AGPL / GPL 代码**。快速和弦分析不再用 essentia.js，改为自研 Rust 实现（色度 + 模板 + Viterbi）。Spike 000 中这一方法与 madmom 并列最高（96.1%），并且能和引擎一样编译到原生和 WASM。ADR-0006 已同步修改。
4. **非商用许可的东西不进入主程序，也不进入仓库中的代码**：
   - madmom 模型只由可选组件在用户机器上下载使用，仓库不包含模型文件；
   - 由 Chordonomicon 统计出的概率表放在 `data/third_party/`，单独标注 CC BY-NC 4.0；
   - 两者都列在 `THIRD_PARTY_NOTICES.md`。
5. **CI 用 cargo-deny 守护许可证白名单**（`deny.toml`），引入不在白名单内的依赖会直接失败。

## 备选方案

| 方案 | 不选的原因 |
| --- | --- |
| GPL-3.0 / AGPL-3.0 | 能直接用 essentia.js，但限制他人复用；而自研快速分析同样准确 |
| 只用 MIT | 没有 Apache-2.0 的专利授权条款 |
| 不设许可（保留所有权利） | 仓库公开却不能合法复用，不符合开源目的 |

## 后果

- 将来若转为商用，只需替换两个非商用组件（分析模型、推荐数据），主程序代码无需重新授权。
- 新增待办：Rust 快速和弦分析器（替代 essentia.js），排在 S4。
