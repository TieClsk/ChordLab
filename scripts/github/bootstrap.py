#!/usr/bin/env python3
"""Seed a GitHub repo with ChordLab's labels, sprint milestones and backlog issues.

Idempotent: existing labels/milestones/issues (matched by title) are skipped.
Usage:  python scripts/github/bootstrap.py OWNER/REPO [--dry-run]
Requires the GitHub CLI (`gh`) authenticated with repo scope.
"""
import json
import subprocess
import sys

REPO = sys.argv[1]
DRY = "--dry-run" in sys.argv


def gh(*args, input=None):
    if DRY:
        print("DRY gh", " ".join(args)[:160])
        return "[]"
    r = subprocess.run(["gh", *args], input=input, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.strip())
    return r.stdout


LABELS = {
    "type:story": ("1d76db", "用户故事 / user story"),
    "type:bug": ("d73a4a", "缺陷 / bug"),
    "type:spike": ("8250df", "技术验证 / spike"),
    "type:chore": ("cfd3d7", "工具、CI、依赖"),
    "type:docs": ("0075ca", "文档"),
    "type:tech-debt": ("fbca04", "技术债"),
    "epic:E0-theory": ("c5def5", "乐理核心"),
    "epic:E1-piano": ("c5def5", "互动钢琴"),
    "epic:E2-lessons": ("c5def5", "和弦教程"),
    "epic:E3-analysis": ("c5def5", "音乐分析"),
    "epic:E4-sound": ("c5def5", "音色"),
    "epic:E5-arrange": ("c5def5", "编曲"),
    "epic:infra": ("c5def5", "基础设施"),
    "prio:must": ("b60205", "MoSCoW: Must"),
    "prio:should": ("d93f0b", "MoSCoW: Should"),
    "prio:could": ("fbca04", "MoSCoW: Could"),
    "status:ready": ("0e8a16", "满足就绪定义"),
    "status:blocked": ("000000", "被阻塞"),
    "backlog": ("ededed", "产品待办"),
}

MILESTONES = [
    ("S0", "2026-11-22", "基础架构：仓库、流程、CI、引擎验证、EngineBridge、前端骨架"),
    ("S1", "2026-12-06", "互动钢琴 MVP（桌面）"),
    ("S2", "2026-12-20", "和弦推荐 + 进行循环 + 音色"),
    ("S3", "2027-01-03", "和弦教程 v1（双语）"),
    ("S4", "2027-01-17", "分析 v1：快速分析 + BTC/ONNX 验证"),
    ("S5", "2027-01-31", "分析 v2：精确分析 + 记录管理"),
    ("S6", "2027-02-14", "歌单规律 + 歌曲示例 + Web 版发布"),
    ("S7", "2027-02-28", "编曲 I：钢琴卷帘 + 鼓机"),
    ("S8", "2027-03-14", "编曲 II：调音台 + 导出"),
    ("Later", None, "尚未排期"),
]


def story(story, criteria, notes=""):
    body = f"### 用户故事 / Story\n{story}\n\n### 验收标准 / Acceptance criteria\n" + "\n".join(f"- [ ] {c}" for c in criteria)
    if notes:
        body += f"\n\n### 技术备注 / Technical notes\n{notes}"
    return body


# (title, labels, milestone, body, closed)
ISSUES = [
    ("[Spike] SPK-1 引擎架构验证（Rust 双目标、AudioWorklet、Tauri、采样、导出）", ["type:spike", "epic:infra", "prio:must"], "S0",
     "已完成，结论见 docs/spikes/001-engine-architecture.md 与 ADR 0001–0006。", True),
    ("[Chore] INF-3 CI：fmt / clippy / test / WASM 一致性 / 桌面编译", ["type:chore", "epic:infra", "prio:must"], "S0",
     story("作为开发者，我希望每个 PR 自动检查，以便 main 始终可运行。", ["工作流在 PR 上运行并全绿", "原生与 WASM 输出不一致时失败", "在 main 上开启分支保护，要求 CI 通过"]), False),
    ("[Story] INF-1 EngineBridge 接口与桌面 / Web 两种实现", ["type:story", "epic:infra", "prio:must", "status:ready"], "S0",
     story("作为前端开发者，我希望通过一个统一接口驱动引擎，以便同一段界面代码在桌面和浏览器都能发声。",
           ["TS 接口：loadProject / command / status / addSample", "TauriBridge：invoke engine_cmd，状态轮询", "WorkletBridge：SAB 环形缓冲 + 状态 SAB", "示例页面在两端都能弹出声音"],
           "协议见 ADR-0003；worklet 中无 TextEncoder，工程 JSON 在主线程编码。"), False),
    ("[Story] INF-2 Web 前端骨架：React + Vite + TypeScript + i18next（中/英）", ["type:story", "epic:infra", "prio:must", "status:ready"], "S0",
     story("作为用户，我希望界面支持中英文切换，以便用我习惯的语言学习。", ["apps/web 可在 Vite 下运行，并被 Tauri 加载", "所有文本走翻译键，提供 zh-CN 与 en", "语言切换即时生效并被记住", "ESLint + Vitest 接入 CI"]), False),
    ("[Spike] SPK-3 真机音频延迟与稳定性（macOS / Windows）", ["type:spike", "epic:infra", "prio:must"], "S0",
     "在开发者电脑上运行 `cargo run --release -p chordlab-desktop` 与 `chordlab-engine bench`，记录：输出设备、缓冲大小、回调数/秒、最大回调耗时、主观延迟。容器内的 ALSA 空设备无法验证这些。", False),
    ("[Story] INF-4 采样存储支持 i16 与立体声", ["type:story", "epic:infra", "prio:must"], "S1",
     story("作为用户，我希望钢琴音色占用更少内存且保留立体声。", ["采样可用 i16 存储，内存减半", "立体声采样左右声道独立播放", "原生 / WASM 一致性测试仍通过"], "ADR-0005"), False),
    ("[Story] INF-5 构建期 Lite 钢琴采样包脚本", ["type:story", "epic:E4-sound", "prio:must"], "S1",
     story("作为用户，我希望打开钢琴页时快速加载出好听的钢琴。", ["脚本从 Splendid Grand Piano 生成 Lite 包：每 3 半音 1 个采样、2 个力度层、6 秒截断淡出", "加载后内存 ≤ 60 MB", "包体积与加载时间写入文档"], "公有领域 AKAI 采样；ADR-0005"), False),
    ("[Story] E0-1 统一表示音符、和弦、调、级数", ["type:story", "epic:E0-theory", "prio:must"], "S1",
     story("作为学习者，我希望系统准确地说出和弦的组成和功能。", ["“Am7” → A C E G，在 C 调中为 vi7", "和弦识别结果按常用度和当前调排序（E G C 首选 C/E）", "级数用常见写法（vi，而非 VIm）"], "封装 tonal；Spike 000 记录了三个坑"), False),
    ("[Story] E1-1 用鼠标 / 电脑键盘弹奏钢琴", ["type:story", "epic:E1-piano", "prio:must"], "S1",
     story("作为学习者，我希望在屏幕上弹钢琴。", ["至少 2 个八度，可移动八度", "鼠标拖奏、电脑键盘映射", "多键同按不丢音"]), False),
    ("[Story] E1-2 点击和弦发声并高亮琴键", ["type:story", "epic:E1-piano", "prio:must"], "S1",
     story("作为学习者，我希望点一个和弦就能听到并看到它的组成音。", ["根音与其他音不同颜色", "显示转位，可切换", "名称与级数中英双语显示"]), False),
    ("[Story] E4-1 切换音色（≥ 3 种）", ["type:story", "epic:E4-sound", "prio:must"], "S2",
     story("作为用户，我希望换不同的音色来弹奏。", ["Lite 钢琴、电钢（合成）、合成 Pad 至少 3 种", "切换无爆音", "首次加载显示进度"]), False),
    ("[Story] E1-3 推荐下一个和弦", ["type:story", "epic:E1-piano", "prio:must"], "S2",
     story("作为创作者，我希望点击和弦后看到常见的下一个和弦。", ["至少 3 个推荐，带级数和概率", "一键试听", "推荐基于级数、与调无关"], "离线统计 Chordonomicon（CC BY-NC，非商用可用）生成转移概率 JSON"), False),
    ("[Story] E1-4 和弦进行循环播放", ["type:story", "epic:E1-piano", "prio:should"], "S2",
     story("作为创作者，我希望把和弦串成进行循环播放。", ["可调 BPM", "柱式 / 分解两种节奏型", "进行保存为工程中的 Clip"], "ADR-0004"), False),
    ("[Story] E2-1 和弦原理课程（双语）", ["type:story", "epic:E2-lessons", "prio:must"], "S3",
     story("作为入门者，我希望循序渐进地理解和弦。", ["音程 → 三和弦 → 七和弦 → 调内和弦 → 功能 共 5 课", "每课讲解 + 可弹演示 + 小测验", "中英双语"]), False),
    ("[Story] E2-2 常见和弦进行互动演示", ["type:story", "epic:E2-lessons", "prio:must"], "S3",
     story("作为学习者，我希望听到常见进行在不同调上的样子。", ["I–V–vi–IV、ii–V–I、卡农、4536 至少 4 个", "可移调", "在钢琴上逐个高亮播放"]), False),
    ("[Story] E3-1 上传单个或多个音频文件", ["type:story", "epic:E3-analysis", "prio:must"], "S4",
     story("作为扒曲者，我希望一次拖入多首歌。", ["支持 MP3/WAV/FLAC/M4A", "批量上传带进度", "文件只在本机处理"]), False),
    ("[Story] E3-2 识别调、BPM、和弦并同步播放", ["type:story", "epic:E3-analysis", "prio:must"], "S4",
     story("作为扒曲者，我希望看到和弦时间轴并跟着音乐走。", ["快速分析在 10 秒内完成（4 分钟歌曲）", "时间轴与播放同步，点击跳转", "可手动改调、BPM ×2 / ÷2"], "自研 Rust 快速分析：色度 + 模板 + Viterbi，原生与 WASM 双目标（ADR-0006、ADR-0007）"), False),
    ("[Story] ANA-1 Rust 快速和弦分析器（替代 essentia.js）", ["type:story", "epic:E3-analysis", "prio:must"], "S4",
     story("作为用户，我希望分析在本机快速完成，且主程序不含 AGPL 代码。",
           ["新 crate `analysis`：色度（CQT）+ 24 类模板 + Viterbi 平滑，另含调与 BPM 估计", "原生与 wasm32 都能编译，Web 端在 Worker 中运行", "在 Spike 000 合成测试集上 maj-min ≥ 95%，4 分钟歌曲 < 10 s",
            "用 mir_eval 结果做回归测试"], "移植 experiments/chord-recognition/eval_chords.py 中的方法 A；ADR-0007"), False),
    ("[Spike] SPK-2 BTC → ONNX → Rust 推理", ["type:spike", "epic:E3-analysis", "prio:should"], "S4",
     "问题：能否去掉 Python sidecar？导出 BTC（MIT）为 ONNX，用 ort / tract 在 Rust 中推理；测准确率、速度、体积。时间盒 2 天。", False),
    ("[Story] E3-3 级数化并统计高频进行", ["type:story", "epic:E3-analysis", "prio:must"], "S5",
     story("作为创作者，我希望知道一首歌用了哪些套路。", ["和弦转为级数", "Top-N 进行模式及次数"]), False),
    ("[Story] E3-5 管理分析记录、手动修正和弦", ["type:story", "epic:E3-analysis", "prio:should"], "S5",
     story("作为扒曲者，我希望修正识别错误并管理记录。", ["重命名、标签、搜索、删除", "修正后统计更新", "导出 JSON / MIDI"]), False),
    ("[Story] E3-4 歌单 / 系列级规律对比", ["type:story", "epic:E3-analysis", "prio:should"], "S6",
     story("作为创作者，我希望对比一个歌单的整体和弦习惯。", ["常用和弦、常用进行、调分布"]), False),
    ("[Story] E2-3 进行配流行歌曲示例", ["type:story", "epic:E2-lessons", "prio:should"], "S6",
     story("作为学习者，我希望知道哪些歌用了这个进行。", ["显示歌名、歌手、段落、和弦与级数", "不包含原曲音频与歌词"]), False),
    ("[Story] E5-1 钢琴卷帘编辑 MIDI 片段", ["type:story", "epic:E5-arrange", "prio:should"], "S7",
     story("作为创作者，我希望用钢琴卷帘写旋律和伴奏。", ["增删改音符、拖动、调长度与力度", "量化与网格吸附", "撤销 / 重做"]), False),
    ("[Story] E5-2 鼓机网格编写节奏", ["type:story", "epic:E5-arrange", "prio:should"], "S7",
     story("作为创作者，我希望快速写鼓。", ["16 步网格", "底鼓、军鼓、踩镲等鼓件", "可调力度"]), False),
    ("[Story] E5-3 调音台", ["type:story", "epic:E5-arrange", "prio:should"], "S8",
     story("作为创作者，我希望调整每条轨道的音量和效果。", ["音量、声像、静音、独奏、发送效果", "实时生效无爆音"]), False),
    ("[Story] E5-4 导出音频", ["type:story", "epic:E5-arrange", "prio:should"], "S8",
     story("作为创作者，我希望把作品导出成音频文件。", ["导出 WAV（之后 MP3）", "离线渲染与实时播放一致"]), False),
    ("[Tech debt] TD-1 大工程增量更新", ["type:tech-debt", "epic:infra", "prio:should"], "S7",
     "超过约 3 万音符时整体重载 > 5 ms（tests/scale.rs）。Web 端在 Worker 中编译事件表再传入 worklet，或按轨道增量更新。ADR-0004。", False),
    ("[Story] E5-5 合成器参数编辑与预设", ["type:story", "epic:E5-arrange", "prio:could"], "Later", story("作为创作者，我希望调出自己的合成器音色。", ["参数面板", "保存 / 载入预设"]), False),
    ("[Story] E5-6 导入采样 / SFZ 乐器并编辑", ["type:story", "epic:E5-arrange", "prio:could"], "Later", story("作为创作者，我希望用自己的采样做乐器。", ["导入 WAV/FLAC/SFZ", "设置根音、范围、力度层"]), False),
    ("[Tech debt] TD-2 桌面端采样磁盘流式读取", ["type:tech-debt", "epic:E4-sound", "prio:could"], "Later", "Full 钢琴包（259 MB）上线前需要。ADR-0005。", False),
    ("[Story] E1-5 MIDI 键盘输入", ["type:story", "epic:E1-piano", "prio:could"], "Later", story("作为有 MIDI 键盘的用户，我希望直接用键盘弹。", ["Web MIDI（Chromium）/ midir（桌面）", "识别按下的和弦"]), False),
    ("[Story] E3-6 上传 MIDI 文件直接分析", ["type:story", "epic:E3-analysis", "prio:could"], "Later", story("作为用户，我希望分析 MIDI 文件里的和弦。", ["无需音频识别，准确率接近 100%"]), False),
]


def main():
    existing_labels = {l["name"] for l in json.loads(gh("label", "list", "-R", REPO, "--limit", "200", "--json", "name") or "[]")}
    for name, (color, desc) in LABELS.items():
        if name not in existing_labels:
            gh("label", "create", name, "-R", REPO, "--color", color, "--description", desc)
    existing_ms = {m["title"]: m["number"] for m in json.loads(gh("api", f"repos/{REPO}/milestones?state=all&per_page=100") or "[]")}
    for title, due, desc in MILESTONES:
        if title not in existing_ms:
            args = ["api", f"repos/{REPO}/milestones", "-f", f"title={title}", "-f", f"description={desc}"]
            if due:
                args += ["-f", f"due_on={due}T23:59:59Z"]
            gh(*args)
    existing_issues = {i["title"] for i in json.loads(gh("issue", "list", "-R", REPO, "--state", "all", "--limit", "500", "--json", "title") or "[]")}
    for title, labels, ms, body, closed in ISSUES:
        if title in existing_issues:
            continue
        labels = labels + ([] if closed else ["backlog"])
        url = gh("issue", "create", "-R", REPO, "--title", title, "--body", body, "--milestone", ms, *sum((["--label", l] for l in labels), [])).strip()
        print("created", url or title)
        if closed and url:
            gh("issue", "close", url, "--reason", "completed")


if __name__ == "__main__":
    main()
