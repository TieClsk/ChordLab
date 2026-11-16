# Spike 000 · 和弦识别与乐理库选型

- 日期：2026-11-15
- 代码：`experiments/chord-recognition/`

## 方法

合成 8 首流行风格曲目（约 6 分钟，带精确标注；钢琴类加法合成 + 低音 + 带经过音的旋律 + 噪声打击乐；随机转位；8 个调；72–128 BPM），用 mir_eval 计算 maj-min 加权准确率。

> 合成音频比真实歌曲容易得多，结果只用于横向比较。论文中深度模型在真实歌曲上约 82–83%。

## 结果

| 方案 | maj-min 准确率 | 最难曲目 | 实时率 | 运行环境 | 许可证 |
| --- | --- | --- | --- | --- | --- |
| madmom CNN+CRF | 96.1% | 86.4% | 0.091 | Python | 代码 BSD，模型 CC BY-NC-SA |
| librosa 色度 + 模板 + Viterbi | 96.1% | 95.5% | 0.074 | Python | ISC |
| madmom DeepChroma+CRF | 94.1% | 82.9% | 0.003 | Python | 同 madmom |
| essentia.js ChordsDetection | 91.4%（逐帧口径） | 82.7% | 0.008 | 浏览器 / Node | AGPL-3.0 |
| Essentia（Python） | 88.3% | 73.9% | 0.012 | Python | AGPL-3.0 |

- 调性检测 7/8 正确（错误为关系小调）；BPM 7/8 正确（错误为倍速）。
- tonal 6.6：约 17 µs/次；需要自定义和弦排序与级数格式化。**已解决（#10）**：`@chordlab/theory` 封装了 tonal，三个坑都有测试覆盖，并在 12 个大调的全部顺阶和弦与转位上做了全面检查。

## 复现

```bash
python experiments/chord-recognition/make_testset.py <out_dir>
python experiments/chord-recognition/eval_chords.py <out_dir>
```
