# 敏捷流程 / Agile process

## 角色 / Roles

| 角色 | 由谁担任 | 职责 |
| --- | --- | --- |
| 产品负责人 (PO) | 项目发起人 | 排定待办优先级、验收演示 |
| 开发团队 | 发起人 + Claude | 估算、实现、测试、写文档 |

## 工件 / Artifacts

| 工件 | 位置 |
| --- | --- |
| 产品待办列表 Product Backlog | GitHub Issues（标签 `backlog`，按优先级排序）+ Project 看板 |
| Sprint 待办 | 当期里程碑 `S<n>` 下的 Issue |
| 增量 Increment | 每个 Sprint 末 `main` 上的标签 `v0.<n>.0` |
| 需求与愿景 | [docs/product/](../product/) |
| 架构与决策 | [docs/architecture/](../architecture/)、[docs/adr/](../adr/) |
| 技术验证结论 | [docs/spikes/](../spikes/) |
| Sprint 记录 | [docs/sprints/](../sprints/) |

## 就绪定义 / Definition of Ready

一个 Issue 可以进入 Sprint，当且仅当：

1. 有清晰的用户故事或问题陈述；
2. 有可检验的验收标准；
3. 估算 ≤ 5 点（更大的先拆分）；
4. 依赖已解除，或标了 `status:blocked` 并写明原因。

## 估算 / Estimation

用故事点（1, 2, 3, 5, 8）。8 点必须拆分。点数记在 Issue 标题后缀 `[3]` 或 Project 字段里。

## 看板列 / Board columns

`Backlog` → `Ready` → `In progress` → `In review` → `Done`

## 每个 Sprint 的仪式 / Ceremonies

| 时间 | 活动 | 产出 |
| --- | --- | --- |
| 第 1 天 | 计划会 | 里程碑内 Issue 列表、Sprint 目标写入 `docs/sprints/sprint-XX.md` |
| 每天 | 进展记录（异步） | 在 Sprint 文档追加一行：做了什么 / 下一步 / 阻塞 |
| 最后一天 | 评审演示 | 按“可演示成果”验收，录屏或截图存档 |
| 最后一天 | 回顾 | 保留 / 改进 / 尝试，各 1–3 条 |

## 技术验证 / Spikes

- 每个 Spike 有时间盒（通常 ≤ 2 天）和要回答的具体问题。
- 结论写入 `docs/spikes/NNN-<名称>.md`：问题、方法、数据、结论、对架构的影响。
- 影响架构的结论要落成 ADR。
