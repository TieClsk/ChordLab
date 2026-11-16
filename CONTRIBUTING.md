# 贡献与开发流程 / Contributing

本文件是 ChordLab 的开发约定，所有改动（包括 AI 助手提交的）都遵守它。
This file defines how work flows through the ChordLab repo. It applies to every change, human or AI.

## 1. 敏捷节奏 / Agile cadence

- 每个 Sprint 2 周（S0 为 1 周），对应 GitHub 里程碑 `S0`、`S1` …
- Sprint 开始：从 `backlog` 里挑选满足“就绪定义”的 Issue，放进当期里程碑。
- Sprint 结束：演示“可演示成果”，在 `docs/sprints/sprint-XX.md` 写评审与回顾，未完成的 Issue 移到下一里程碑。
- 详细规则见 [docs/process/agile.md](docs/process/agile.md)。

## 2. 分支模型 / Branching (GitHub Flow)

| 分支 | 用途 | 规则 |
| --- | --- | --- |
| `main` | 随时可运行、可演示 | 受保护：只能通过 PR 合并，CI 必须通过，禁止 force push |
| `feat/<issue号>-<简述>` | 新功能 / 用户故事 | 从 `main` 拉出，生命周期 ≤ 1 个 Sprint |
| `fix/<issue号>-<简述>` | 缺陷修复 | 同上 |
| `spike/<issue号>-<简述>` | 技术验证 | 结论写入 `docs/spikes/`，代码可合并到 `experiments/` 或丢弃 |
| `docs/<简述>` | 纯文档 | 可小步合并 |
| `chore/<简述>` | 工具、CI、依赖 | — |

- 合并方式：**Squash merge**，PR 标题即最终提交信息。
- 合并后删除分支。
- 每个 Sprint 结束在 `main` 打标签 `v0.<sprint>.0`，并写 `CHANGELOG.md`。

## 3. 提交信息 / Commit messages (Conventional Commits)

```
<type>(<scope>): <摘要>

<正文，可选>

Refs #<issue>
```

`type`：feat / fix / docs / refactor / perf / test / chore / spike（TDD：`test` 提交在 `feat` 之前）
`scope`：engine / theory / web / desktop / analyzer / docs / ci …

## 4. Issue 管理 / Issues

- 一切工作都从 Issue 开始；模板见 `.github/ISSUE_TEMPLATE/`。
- 标签体系：
  - 类型 `type:story` `type:bug` `type:spike` `type:chore` `type:docs` `type:tech-debt` `type:ux-test` `type:ux-feedback`
  - 史诗 `epic:E0-theory` `epic:E1-piano` `epic:E2-lessons` `epic:E3-analysis` `epic:E4-sound` `epic:E5-arrange` `epic:infra`
  - 优先级 `prio:must` `prio:should` `prio:could`
  - 状态 `status:ready` `status:blocked`
- 用户故事必须写验收标准；Spike 必须写要回答的问题和时间盒。

## 5. 完成定义 / Definition of Done

开发方式为 **TDD**：先提交失败的测试，再实现，再重构。详见 [docs/process/testing.md](docs/process/testing.md)。

- [ ] 验收标准全部满足：可自动化的有测试，其余写进本 Sprint 的 UX 测试脚本
- [ ] PR 提交历史体现 测试 → 实现 → 重构；新逻辑有测试，CI 绿
- [ ] 预览安装包构建成功（用于 UX 测试）
- [ ] 用户可见文本已有中英文
- [ ] 相关文档已更新（架构变化 → ADR；新模块 → README）
- [ ] PR 已关联 Issue（`Closes #n`）

## 6. 许可 / License

本项目代码采用 **MIT OR Apache-2.0** 双许可（[ADR-0007](docs/adr/0007-licensing.md)）。提交贡献即表示你同意按同一双许可授权你的贡献。
新增 Rust 依赖必须通过 `cargo deny check licenses`；不得引入 GPL / AGPL 代码到主程序。

## 7. 本地开发 / Local setup

见 [docs/dev/setup.md](docs/dev/setup.md)。
