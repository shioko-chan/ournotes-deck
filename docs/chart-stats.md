# 谱面统计

[English](chart-stats.en.md)

`ournotes-deck chart-stats` 测量 deck data 文件中每张谱面的统计。Python 包 `ournotes-sim` 与 Rust 库 `ournotes_sim::chartstats` 使用同一模型。

```sh
ournotes-deck chart-stats --data deck-data.json [--seeds 8] [--charts ID,...] [--jobs N] \
  [--stats-cache DIR] -o chart-stats.json
```

谱面统计格式为 `ournotes-deck.chart-stats/3`。激走开启时给出各次抽签、技能判定按名义概率独立的期望，
每个估计以 `[中心, 区间半宽]` 表示，包含概率运算和分数取整的外向误差界。打法为准点判定、Just 任务区间内 Just、
其余 Perfect、每段名次 1；另测全 Perfect 打法。该概率模型与给定种子的单局回放分别计算。
序列化分数中心保留三位小数，抽签次数保留五位，权重保留十二位；半宽在相同精度上向外取整，并包含中心的舍入误差。

普通加分效果的概率条件在原条件检查位置逐次分支，保留短路顺序、计数和触发历史。
当其余条件具有可证明的确定性日程、效果只写分数时，各分支复用同一 LUCK 概率曲线，按事件概率加权完整分数和名次区间。
所有正质量分支完成后发布统计；事件树超出工作额度时返回 `Capacity` 错误。

`--charts` 选择 score id，按输入数据中的谱面顺序输出；`--jobs N` 同时测 N 张谱面，结果与逐张测量相同。
`--seeds N` 指定幸运谱面的回放种子数量，默认 8，与期望统计无关；没有幸运区间时回放种子为 `[0]`。
`--no-gekisou-aptitude` 关闭单技能适性，保留基线统计。

`--stats-cache DIR` 持久保存各个已编译测量程序及抽签曲线的原始结果，分别复用基线、普通技能位置、
激走变体与交叉项。每项完成后原子保存，可在中断后继续；输入的元数据、kind / shape 目录和校验卡组每次重新生成。
缓存按模型源码和格式隔离，输出与无缓存测量一致；命中与计算计数写到 stderr。
存储格式、依赖边界与 Rust API 见 [谱面统计缓存](chart-stats-cache.md)。

每张谱面的 `expectation` 包含无技能 `score`、`scorePerfect`、区间结果、普通加分技能的 `weights[kind][position]`、
`rangeWeights[kind][position][range]`，以及随机卡组的 `check` / 固定名次的 `rankCheck`。
区间分数、名次奖励、幸运点数和四种抽签结果次数为期望区间；最大连击和 Just 数在证明与抽签独立后用整局模拟精确取得。
`rankBonusPerfect` 是 Perfect 打法的名次奖励期望。超过三段 fever 时激走记为 `unplayable`、`expectation` 为 null；
激走关闭的自由 Live 仍由 `offSeeds` 给出确定性分数和权重。

在不重叠、没有读取确定名次的条件的范围内，普通技能权重在其他名次下按
`weight_r = weight + Σ (p_i(r_i) − p_i(1)) / 100 × rangeWeight_i` 变换。
无技能总分为 `score − Σ rankBonus_i + Σ E[trunc(rangeScore_i × p_i(r_i) / 100)]`；
期望奖励可由区间得分期望乘百分比并加取整余量包围，不能把期望中心截断当作奖励期望。
每个 `check` 包含 `deck`、`ranks`、`expected`、`predicted`、`bound`，在另一综合力下用完整 DP 校验线性预测；
两个区间端点的最大距离超过界限即报错。读取确定名次的 kind 没有线性区间权重。

单技能适性由文件级 `gekisouAptitude` 的 `plainKind`、`host`、`law`、`shapes` 与每张谱面的 `factors`、`variants` 给出。
形状按来源、任务与效果参数去重；成员技能取最高等级，小卡支援技能取最高突破的等级。
仅差乐队目标的支援技能共享形状，保留各技能的成员目标和乐队，并分别测匹配、不匹配两种情况。
支援技能使用同任务的合成空成员技能作为宿主。每次只带一个形状，按单人第 1 名测量，增量由「有技能期望 − 无技能期望」得到。

变体包含 `score`、`scorePerfect`、`tail`、`tailPerfect`、`converted`、各区间增量、普通 kind 的交叉权重和期望校验。
全部数值用 `[中心, 区间半宽]` 表示；证明独立的计数半宽为 0。
`tail = Δscore − Σ(ΔrangeScore + ΔrankBonus)` 包含技能延续到区间外的收益，Perfect 端使用对应 Perfect 字段。
`factors.lotteries` 表示基线消耗的抽签次数期望。交叉权重测普通技能因子与该形状的相互作用。
`rangeWeights` 适用于区间线性、且形状的触发、条件、释放、重置和累计判定都独立于确定名次（7012）的情况，其他情况为 null。
读取确定名次的形状保留第 1 名的得分增量和普通技能交叉权重，`check` 按各段第 1 名校验，测量值适用于该名次场景。
在线性名次域内，`check` 使用给定的固定名次与完整期望对照。

多个形状的增量不能相加来估计完整编成，连击封顶、幸运槽、Rush 与 Just 触发会互相影响。
普通技能的交叉权重只测最佳打法；低于 100% Just 时仅能插值无普通技能的 Just / Perfect 端点。
Just 率插值和 Great 比例缩放是统计摘要的近似使用方式，完整逐音符结果通过共享 `replay` API 按帧和种子计算。
得分证明区间要求效果状态标识互不冲突。持续的直接得分效果在正数音乐长度之后，需要逐原始帧证明尾部保持未激活、
原生执行阶段顺序一致且录制时钟完整。超出这些支持域或完整顺序枚举容量的输入会返回错误。

库调用使用 `chart_stats_with` / `document_with` 与 `Options { replay_seeds, aptitude }`。
`aptitude: false` 关闭适性；无谱面的输入也可生成形状表，或调用 `aptitude_header(master, kinds)`。
