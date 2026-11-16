# analyzer — 精确和弦识别（可选组件）

基于 madmom CNN+CRF。按 [ADR-0006](../../docs/adr/0006-chord-analysis-deployment.md)，桌面版首次使用“精确分析”时下载，以常驻进程运行。

> 许可提示：madmom 预训练模型为 CC BY-NC-SA 4.0（非商用）。本项目为个人非商用，可以使用。

## 现状（Spike 原型）

```bash
python -m venv .venv && . .venv/bin/activate
pip install numpy scipy "Cython>=0.29" && pip install --no-build-isolation git+https://github.com/CPJKU/madmom
python analyze.py song.wav   # -> {"chords": [[start, end, "C:maj"], ...], "sec": ...}
```

## 打包（PyInstaller）

```bash
pyinstaller --noconfirm --onedir --collect-data madmom --collect-submodules madmom \
  --collect-submodules numpy.core --hidden-import numpy.core.multiarray --name analyzer analyze.py
```

实测：198 MB（tar.gz 90 MB），冷启动约 7 s。后续改为常驻 HTTP 服务；并评估 BTC→ONNX→Rust 以去掉 Python（SPK-2）。
