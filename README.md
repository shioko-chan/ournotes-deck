# ournotes-deck

BanG Dream! Our Notes 的综合力、跳过分数与演出分数计算，以及精确的 Top-K 组卡搜索。

[English](README.en.md) · [路线图](docs/roadmap.md)

> [!NOTE]
> **维护状态：不再积极维护，欢迎接手。**
>
> 维护者没有精力继续投入，而且实际上并不游玩本作，因此本仓库今后无法保证任何实现的正确性与效率。欢迎直接 fork 或提交 PR，改动规模不限，无需事先讨论。需要发布新版本时，可在 main 上推送与版本号一致的 `vX.Y.Z` tag，由发布流程构建并发布 GitHub Release 与 PyPI 包 `ournotes-sim`；需要写权限请开 issue。仓库以 MIT OR Apache-2.0 发布，接手与再发布都不需要额外许可。
>
> 就目前核对过的游戏版本（客户端 1.0.6）而言，各项计算（综合力、跳过分数、活动点数）与整场演出模拟（演出分数）的结果是正确的，搜索在返回 `Complete` 时也仍是所声明模型与输入上的精确 Top-K（见下文「正确性」）。实现以正确性为先，效率不是重点：搜索与单局模拟基本是单线程的，并行搜索在路线图里也只是可选的后备方案，因此较慢，也可能达到时间上限。以上结论只覆盖核对过的版本、资源与输入；游戏更新、未建模的规则和超出声明范围的结果都不在保证之内。

## 仓库布局

仓库是一个 Cargo workspace，含两个 crate，搜索依赖模型，模型不依赖搜索：

| crate | 内容 |
| --- | --- |
| `crates/ournotes-sim` | 模型：综合力、跳过分数、演出分数与整场模拟（`live`）、活动点数（`event`）、场景（`scenario`）、名册解析成的卡池（`pool::{Pool, Deck}`）、deck data 文件（`data`）、谱面统计（`chartstats`）、单局回放（`replay`） |
| `crates/ournotes-search` | 精确 Top-K 组卡搜索与推荐入口（下文“推荐主链”的模块），以及统一命令行程序 `ournotes-deck` |

只需要计算或模拟（例如谱面统计、回放）时依赖 `ournotes-sim`；要搜索时依赖 `ournotes-search`，模型类型仍从 `ournotes-sim`
引用。`wasm/replay` 是基于 `ournotes-sim` 的回放 WASM，`wasm/recommend` 是基于 `ournotes-search` 的推荐 WASM，
`python` 是 Python 包 `ournotes-sim`（`ournotes-sim` 的谱面统计，见 [python/README.md](python/README.md)），
`tools/search-harness` 是搜索的测量工具；这四个是 workspace 之外的独立 package，各有自己的 `Cargo.lock`。
两个 crate 都有 `search-diagnostics` 特性；`ournotes-search` 的这个特性会同时打开 `ournotes-sim` 的。

## 推荐主链

建池与搜索分为两层：

`DeckData + Roster + RecommendationRequest → handler::build_card_pool → BuiltProblem → search::recommend_built`

| 模块 | 职责 |
| --- | --- |
| `engine` | `recommend` / `recommend_json`，串联建池与搜索 |
| `types` | 请求、目标、随机条件、完成状态与结果类型 |
| `handler` | 解析场景、培养与目标，校验执行条件，构建只读 `BuiltProblem` / `SearchContext` |
| `domain` | 合法成员／Snap 索引、必须成员、队长和资源唯一性约束 |
| `search` | 消费已建问题，分派 Power/Skip 或队伍求解器，维护预算与 Top-K |
| `auxiliary` | `evaluate_fixed` / `evaluate_built` / `rank_fixed_songs`，复用同一求值链 |

CLI、WASM 和 harness 使用这些入口。
`DeckData` 可加载一次复用。`BuiltProblem` 冻结请求和持有配置，只读借用源数据；重复搜索拥有独立的预算和结果。
一次性 `engine::recommend` 的时限包含建池；对已建问题执行搜索／评分时，时限从执行开始。
结果的 `telemetry`（`ournotes-deck.telemetry/1`，字段见 [埋点说明](docs/telemetry.md)）按阶段记录建池、上界编译与搜索；对已建问题执行时没有建池阶段。

当前统一入口包括含 Snap 技能的 Free Live、声明条件下的 Mission 撃奏、分数与客户端 PT。
Power/Skip 使用专用规范成员集合搜索。实打 Live/PT 以队伍（队长、另外四名成员及各自配对的 Snap）为单位，
按 120 种等概率出场顺序的平均收益排序；除队长外的站位只是排列，不是决策。每支队伍按规范站位给出，
并附 120 种顺序的分数分布和其中最好的顺序。撃奏关闭时，技能的概率判定按不支持拒绝；撃奏中没有 LUCK 区间时，
概率判定控制不了任何能发生的效果，每种顺序仍只有一个确定分数；有 LUCK 区间时，每种顺序按原生抽签概率下的期望分数取收益，台阶和达标类收益相同时按期望分数排序，
并给出带证明的区间排序，区间重叠而无法分出名次时返回 `RefinementRequired`。
默认 `branchAndBound` 在普通 Live 中先搜索成员组合，再搜索 Snap 配对；撃奏保留联合成员／Snap 遍历。
两者都用按位置平均的技能增益给平均分数上界，按非溢出范围内的奖金上界裁剪 PT 分支。
默认预算为 3 秒，候选数不限；调用方可显式设置时间和候选预算，只有完成证明才返回 `Complete`。
无法建立可靠上界时回退完整枚举，原因见 `telemetry.environment.bounds.fallback`。`branchAndBound` 与 `exhaustive` 完成时都证明所给模型和有限随机条件下的 Top-K；`candidate` 仍是启发式结果。
上界的检查与剪枝按深度记在 `telemetry.joint`；逐前缀上界的证明与适用域见 [搜索说明](docs/search.md#uniform-member-order-search)。
成员组合与 Snap 配对的统计在 `telemetry.composition`；[分层搜索证明](docs/search.md#member-compositions-snap-pairings-and-power-frontiers)说明了 Snap 搜索和 PT 分配提前结束的条件。
搜索过程中 `telemetry.proof.globalUpperBound` 给出全域最佳值的上界。
可复现的正确性实验（有限域穷举对拍、各类上界审计、截断审计）及复现方法见[搜索验证](docs/search.md#validation)。
下文的底层 `search` 接口另有契约：每组成员卡只保留一个结果，并选择演出顺序，不同于统一入口的有限随机根目标。

## 计算内容

- **综合力**：逐槽位各项（等级、突破次数与卡面等级带来的卡面属性；角色等级与总角色等级；乐队道具；歌曲属性与标签加成；
  snap 的加成与属性连携；队长技能；VIP；回忆；活动参数）与编成总值，按游戏自身的整数与 binary32 浮点运算，包括其舍入与取整方式。
- **跳过分数**：某张谱面跳过演出时的分数。
- **演出分数**：单个音符分数、帧、连击加成表、倍率指令，以及逐帧的整场演出（`live::full`）：判定转换、连击、血量（回复、护盾、
  归零）、分数计算器（包括迟到判定引起的回退重算），以及演出技能与 snap 技能的条件和效果。撃奏（区间状态、撃奏连击与 Just 计数、幸运抽签、排名加成，以及撃奏技能与撃奏 snap 技能）也已建模；统一入口在声明条件下搜索 Mission 撃奏的分数与 PT。
- **活动点数**：活动加成、评级、加成道具倍率，以及游戏客户端计算的活动点数。实际发放量由游戏服务器决定；本库复现的是客户端自身的计算。
- **搜索**：综合力（可带或不带歌曲、可带或不带活动参数）、跳过分数与演出分数的前 K 个编成，每组 5 张成员卡只保留一个结果。
  演出分数是判定序列下含演出技能与 snap 技能的整场模拟（默认撃奏关闭，`--gekisou` 时按一组 seed 的分数之和排序）；排除 snap 技能时，则是逐音符打法下只含演出技能的分数。
  含 snap 技能时搜索要模拟候选编成，因此较慢；漏判或迟判较多的判定序列可能慢得多，可用时间上限约束这类请求。

## 正确性

**与游戏一致。** 各项计算按游戏客户端的行为逐函数实现，并逐个计算单元与客户端自身的实现对照：以相同输入执行客户端的函数，
按位比较结果（浮点按位比较，整数溢出与抛出异常的路径也一并比较）。对照覆盖：
- 综合力的数值原语与槽位计算；
- 音符分数与帧、连击加成；
- 倍率指令与分数计算器的回退重算；
- 演出技能与 snap 技能的条件和效果；
- 血量、连击与判定；
- 随机数；
- 评级与活动点数。

对照时还运行故意改错的版本，以确认比较确实能发现差异。编成五槽求和与各项加成的构建是纯整数代码，按函数移植。整场演出与客户端
逐帧对照，方法与复现步骤见[原生对照验证](docs/native-validation.md)；重放客户端参考向量的测试在 `native-fixtures` feature 之后。

**搜索精确。** `Complete` 的搜索结果恰为全部合法编成上的规范 Top-K。剪枝只使用在游戏运算下已证明可采纳的上界（证明见
[docs/search.md](docs/search.md)）。搜索结果与独立的穷举实现逐项比较，穷举不共用任何上界、分解或 Top-K 代码；含 snap 技能时，
穷举模拟每一组成员、队长、snap 配置与演出顺序。对拍用例与复现命令见[搜索验证](docs/search.md#validation)。
达到时间上限的搜索返回 `TimedOut`，其中的编成合法且数值精确，但不保证排名。
超出已证明范围的输入、未知卡牌、游戏会拒绝的规则以及尚未建模的部分，都以错误返回。

**共享运行时。** 逐音符计算、谱面统计与搜索共用同一个 Rust 模型；同一个 replay 请求在本机与 WebAssembly 中的结果相同，
比较方法见[原生对照验证](docs/native-validation.md#本机与-webassembly)。

**适用范围。** 核对结论限定于所声明的资源、输入、字段和阶段。逐音符单局结果使用明确的判定、帧序、seed 与排名策略；统计基线及
权重不替代它。

## 数据

本库不含任何游戏数据。它读取一份 deck data 文件（`nnnotes.deck-data/1`，由 `nnnotes deck-data` 命令生成，含同一版本
master 数据的相关表与全部谱面），以及用户的卡牌持有情况：

```json
{
  "player": { "characterRanks": { "1": 20 }, "bandItems": { "101": 10 }, "vipRank": 3, "events": [] },
  "members": [ { "id": 1, "level": 60, "awake": 3, "rank": 2, "liveSkillLevel": 3, "gekisouSkillLevel": 1 } ],
  "snaps": [ { "id": 1, "level": 20, "rank": 1 } ]
}
```

`level` 可以换成 `exp`。谱面按 score id（`MasterLiveMusicScore._id`）选择。

含 snap 技能的演出分数按判定序列模拟：
`{"frames": [0, 16, 33], "judged": [[frame, noteId, judgement, judgementTimeMs]], "baseSeed": 0, "assist": false}`。
`frames` 是每一帧的乐曲时间（毫秒，不减）；`judged` 的每一行表示某个音符在第 `frames[frame]` 帧判定，判定为转换前的判定
（1 Miss、2 Bad、3 Good、4 Great、5 Perfect、6 Just）；血量、连击与技能由模拟得出。默认判定序列为理论最佳：以 60 fps
取帧（`floor(i * 1000 / 60)` 毫秒），直到最后一个判定音符或技能事件之后 2000 毫秒；每个判定音符在第一个到达其谱面时间的帧
判为 Perfect，判定时间取谱面时间（撃奏关闭时游戏不判 Just）。

排除 snap 技能的演出分数默认使用逐音符形式的理论最佳打法：每个判定音符都是 Perfect，全连且不掉血。也可以另给一份打法
`{"notes": [{"noteId", "timeMs", "noteType", "scoreType", "life", "combo"}], "lifeAtEvent": [...], "assist"}`
（判定类型：1 Just、2 Perfect、3 Great、4 Good、5 Bad、6 Miss）。

## 作为库使用

```toml
[dependencies]
ournotes-sim = { git = "https://github.com/empty-sekai/ournotes-deck" }
ournotes-search = { git = "https://github.com/empty-sekai/ournotes-deck" }
```

```rust
use ournotes_search::search::{Constraints, Objective, SearchRequest, search};
use ournotes_sim::cards::Roster;
use ournotes_sim::data::DeckData;
use ournotes_sim::pool::Pool;

let data = DeckData::from_path("deck-data.json").unwrap();
let roster = Roster::from_json(&std::fs::read_to_string("box.json").unwrap()).unwrap();
let pool = Pool::new(&data.master, &roster).unwrap();
let out = search(&pool, &SearchRequest {
    objective: Objective::SkipScore { score_id: 10000103, chart: data.chart(10000103).unwrap() },
    k: 10,
    constraints: Constraints::default(),
    time_limit: None,
})
.unwrap();
for deck in &out.results {
    println!("{:?} {} {:?} {:?}", deck.score, deck.power, deck.members, deck.snaps);
}
```

在 Python 里，PyPI 包 `ournotes-sim` 用同一版本的模型计算 deck data 文件的谱面统计：

```python
import json
import ournotes_sim

stats = json.loads(ournotes_sim.chart_stats(open("deck-data.json", encoding="utf-8").read()))
```

## 命令行

统一入口为 `ournotes-deck`。JSON 推荐请求使用 `recommend` 子命令：

```sh
ournotes-deck recommend --data deck-data.json --roster box.json --request request.json
ournotes-deck recommend --data deck-data.json --snapshot owned-snapshot.json --request request.json
ournotes-deck recommend --help
```

`--roster` 和 `--snapshot` 二选一；`--progress-ms N` 将进度 JSON 行写到标准错误，`-o FILE` 将最终结果写入文件。
按任务指定参数的接口如下：

```sh
ournotes-deck power --data deck-data.json --roster box.json -k 10
ournotes-deck skip  --data deck-data.json --roster box.json --score SCORE_ID -k 10
ournotes-deck live  --data deck-data.json --roster box.json --score SCORE_ID --expectation finite --seed-law law.json [--play stream.json] [--gekisou] -k 10
```

`live` 计入 snap 技能，按 `--seed-law` 给出的有限根种子分布（JSON `[[rootSeed,positiveWeight],...]`）对原生成员顺序求
期望分排序；`--play` 为判定序列，默认理论最佳打法；`--gekisou` 开启激走。场景用 `--scenario`、`--scenario-music`、
`--context` 指定（见 `ournotes-deck --help`）。

约束：`--leader ID`、`--include ID,...`、`--exclude ID,...`、`--exclude-snaps ID,...`、`--no-snaps`、
`--time-limit-ms N`。输出为 JSON。

谱面统计：

```sh
ournotes-deck chart-stats --data deck-data.json [--seeds 8] [--charts ID,...] [--jobs N] \
  [--stats-cache DIR] -o chart-stats.json
```

输出格式为 `ournotes-deck.chart-stats/3`，字段、期望与区间的计算、单技能适性和缓存见[谱面统计](docs/chart-stats.md)。

## 测试

在仓库根目录运行 `cargo test`，即运行两个 crate 的单元测试与集成测试：读取合成的 deck data 文件，并在小规模合成卡池上
将搜索结果与穷举结果逐项比较。两个 WASM package 用 `cargo build --manifest-path wasm/<replay|recommend>/Cargo.toml
--target wasm32-unknown-unknown --release` 构建。Python 包用 `pip install ./python[test]` 构建并安装，
`pytest python/tests` 测试它。
可用 `OURNOTES_DECK_ORACLE_CASES`、`OURNOTES_DECK_ORACLE_SEED0`、`OURNOTES_DECK_ORACLE_MEMBERS`、
`OURNOTES_DECK_ORACLE_SNAPS` 扩大比较规模。含 snap 技能的演出分数可用 `OURNOTES_DECK_SNAPS_CASES`、
`OURNOTES_DECK_SNAPS_SEED0`、`OURNOTES_DECK_SNAPS_MEMBERS`、`OURNOTES_DECK_SNAPS_SNAPS`、`OURNOTES_DECK_SNAPS_NOTES`、
`OURNOTES_DECK_SNAPS_VARIANTS` 扩大比较规模。

## 版本记录

改动见 [CHANGELOG.md](CHANGELOG.md)，由 [git-cliff](https://git-cliff.org/) 根据提交（Conventional Commits）生成。发版时，改版本号的那个提交（workspace、`wasm/*` 与 `python` 的清单及其锁文件）同时重新生成它：`git cliff --tag vX.Y.Z -o CHANGELOG.md`。

## 许可证

MIT OR Apache-2.0。
