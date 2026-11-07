# Chart statistics

[中文](chart-stats.md)

`ournotes-deck chart-stats` measures the statistics of every chart in a deck data file. The Python package `ournotes-sim` and the Rust library `ournotes_sim::chartstats` use the same model.

```sh
ournotes-deck chart-stats --data deck-data.json [--seeds 8] [--charts ID,...] [--jobs N] \
  [--stats-cache DIR] -o chart-stats.json
```

The document uses `ournotes-deck.chart-stats/3`. With Gekisou on, statistics are expectations under independent
nominal probabilities for each lottery and skill activation. An estimate is `[center, interval half-width]`;
the outward interval includes probability arithmetic and score rounding bounds. The play judges every note at
its time, Just inside Just-count ranges and Perfect elsewhere, with rank 1 in every range. The same play with
every Just judged Perfect is measured separately. These expectations and a replay with a supplied seed are
separate calculations.

Serialized score centers retain three decimal places, lottery counts five, and weights twelve. Half-widths round
outward at the same precision and include the center's rounding error. They bound the declared probability
model's expectation.

Probability conditions on ordinary score-up effects branch at each original condition check, retaining the
short-circuit order, counters and trigger history. When the other predicates have a proved deterministic
schedule and the effects write only score, each branch shares the same LUCK probability curve. Complete score
and rank intervals are weighted by the event probabilities. Statistics are published after every positive-mass
skill branch completes; a tree exceeding the work allowance returns a `Capacity` error.

`--charts` selects score ids in the input data's chart order; ids absent from the data are ignored. `--jobs N`
measures N charts at once and writes the same document as one at a time. `--seeds N` controls only `replaySeeds`
for charts with a luck range (default 8, N at least 1); a chart without a luck range has `[0]`. Changing this count
does not change the expectation statistics. Without `-o`, the document goes to standard output.

`--stats-cache DIR` keeps the raw results of every compiled measurement program and lottery curve, reusing the
baseline, ordinary skill positions, Gekisou variants and cross terms separately. Each entry is saved atomically when
it completes, so an interrupted run resumes; the input metadata, the kind / shape catalogues and the check decks are
regenerated on every run. The cache is keyed by the model sources and the format, the output equals an uncached
measurement, and hit and compute counts go to stderr. [Chart statistics cache](chart-stats-cache.md) describes the
storage format, the dependency boundary and the Rust API.

Each chart's `expectation` contains the no-skill `score` and `scorePerfect`, range results, and ordinary score-up
`weights[kind][position]` and `rangeWeights[kind][position][range]`. `kinds` groups the master's ordinary score-up
rows (2000 / 2002 / 2004 / 2005) by type, duration, targets and conditions. A weight measures score gained at factor
1 per unit of deck power. A deck's expected score is approximately
`P × (score / power + Σ factor_k × weights[kind_k][k])`, within the checked flooring bound.

The range results include expected `rangeScore`, rank-1 `rankBonus`, `luckPoints`, and counts of the four lottery
results in `lotResults`. The lottery points and counts are accumulated as moments over the probability states.
The largest Gekisou combo `maxCombo` and the Just count `justCount` are measured exactly by whole-live simulation
after proving their independence from the lotteries. The combo, Just and luck missions rank by these three
indicators, respectively. `rangeScorePerfect` and `rankBonusPerfect` give the corresponding expectations for the
all-Perfect play. A chart with more than three fevers is `unplayable`, and its `expectation` is null.

Within the linear range domain, where a rank bonus does not fall inside another range's score frames and the
kind does not read the confirmed rank, weights at fixed ranks are
`weights[kind][k] + Σ (rankBonusPercents_i[r_i − 1] − rankBonusPercents_i[0]) / 100 × rangeWeights[kind][k][i]`.
The no-skill total is
`score − Σ rankBonus_i + Σ E[trunc(rangeScore_i × rankBonusPercents_i[r_i − 1] / 100)]`.
The target bonus can be enclosed from the expected range score and its percentage with binary32 and truncation
bounds. Truncating the expectation's center does not give the expected bonus. `rangeWeights` is null outside the
linear range domain; a kind that reads the confirmed rank (7012) has a null entry in that array.

`check` validates a random ordinary-skill deck at another power against its full nominal expectation. `rankCheck`
also uses explicit fixed-rank confirmations when linear range weights are available for its deck. Each check
contains `deck`, `ranks`, `expected`, `predicted` and `bound`; the measurement fails if the largest distance between
the intervals' endpoints exceeds the bound.

With Gekisou off, `offSeeds` contains deterministic measurements at seed 0 for every chart, including unplayable
Gekisou charts. The theoretical best play judges every note Perfect at its time, without Just, luck, Gekisou
combo or rank bonus. It gives `score`, ordinary `weights` and a check using the same linear formula. A kind whose
conditions read the Gekisou state has null weights here.

## Gekisou skill aptitude

Statistics also include each skill's aptitude for a chart, without selecting a best formation or changing
`expectation`, `replaySeeds` or `offSeeds`. The file-level `gekisouAptitude` contains `plainKind`, `host`, `law` and
the skill `shapes`; each chart's `charts[].gekisouAptitude` contains range `factors` and the `variants` of its
missions. It is null for a chart with no Gekisou range, one unplayable with Gekisou, or a master without measurable
skills.

```sh
ournotes-deck chart-stats --data deck-data.json -o stats.json
ournotes-deck chart-stats --data deck-data.json --no-gekisou-aptitude -o baseline.json
```

`--no-gekisou-aptitude` skips aptitude measurement; both file-level and per-chart `gekisouAptitude` are null.
Baseline statistics are still produced.

Shapes deduplicate by source, mission and effect parameters. Member skills use their highest level; support skills
use the level at the snap's highest rank, not necessarily the highest level in the effect table. Support skills
that differ only in their band targets share a shape, preserving `skills[].memberTargetIds` / `bandIds`, and are
measured both with `bandMatch: true` and `false`. Each support skill uses a **synthetic, effect-free member Gekisou
skill** of its own mission as host. Each measurement includes one member or support skill alone through the
whole-live engine.

Each variant uses solo rank 1. Its `score`, `scorePerfect`, `tail`, `tailPerfect`, `converted` and range increments use
`[center, interval half-width]`. Score increments subtract the no-skill expectation from the with-skill
expectation at `model.power`. `score` and `scorePerfect` use their respective best-play and Perfect-play
baselines, including their own `rankBonus` and `rankBonusPerfect`. Counts proven independent of the lotteries,
including `converted` judgements, have zero half-width.

`tail = Δscore − Σ(ΔrangeScore + ΔrankBonus)` includes gains outside the range score frames, such as effects
persisting after a range ends. `tailPerfect` uses the corresponding Perfect fields. `factors` lists range note
counts, entering combos and `lotteries`, the expected number of baseline lottery draws.

`weights` gives the change in the plain ordinary kind's weight at each performance position; `rangeWeights`
gives its change within each range. These cross weights measure the interaction of that ordinary skill factor
with the isolated Gekisou shape. Both are null without a plain kind. `rangeWeights` is available for linear ranges
when the shape's trigger, condition, release, reset and cumulative predicates are independent of confirmed rank
(7012); it is null elsewhere. A shape reading confirmed rank retains its rank-1 score increments and ordinary
cross weights. Its `check` uses rank 1. Within the linear rank domain, `check` compares a prediction at fixed ranks
with the full nominal expectation of an ordinary-skill deck at another power, subject to the same interval and
flooring checks. Rank-dependent shape measurements apply to the declared rank-1 scenario.

**Model limits:**

- Only `battleLiveScore` changes, not the separately reported `soloScore`; Free Live has no aptitude gain.
- **Do not add increments of multiple skills.** Gekisou combo saturation, luck gauge/rush interactions and
  Just-count-dependent support triggers can all break additivity.
- The theoretical best play has no Great or Miss: combo protection 12004, Great-to-Perfect 12006 and judgement
  window extension 4004 have zero effect here. Just-count effects 13000/13002 and luck-point effect 11002 can change
  range indicators without increasing score. Shapes of other missions are gated off and omitted on this chart.
- Ordinary-skill factors and rank changes within the linear rank domain use the linear formula, with rank rounding differences checked by
  `check`. Below 100% Just, interpolation of the no-ordinary-skill increment between Just and Perfect plays is
  approximate: conversion 13005, per-Just support 2001 and Just-count effect 13002 are nonlinear. Perfect-play
  cross weights are not measured, so full aptitude with nonzero ordinary skills is unavailable below 100% Just.
  Scaling by `1 − 0.2q` for Great proportion q is also approximate.
- Certified score intervals require distinct effect-state identities. Sustained direct score probes beyond a
  positive music length require an inactive tail proved at every original frame, consistent native phase order
  and a complete recorder clock. Inputs outside these domains or the complete-order work budget return an error.

These interpolation and scaling limits concern statistical summaries. A declared per-note play uses the shared
`replay` API with its actual frame order, skills and seed; see the [model contract](native-validation.en.md#calculation-contract-of-the-shared-model).

Library callers can use `chart_stats_with` / `document_with` with
`Options { replay_seeds: 8, aptitude: true }`; `aptitude: false` disables aptitude measurement.
A `DeckData` without charts still produces the shape header, also available through
`aptitude_header(master, kinds)`.

```rust
use ournotes_sim::{chartstats, data::DeckData, Error};

fn statistics(data: &DeckData) -> Result<serde_json::Value, Error> {
    chartstats::document_with(data, &chartstats::Options { replay_seeds: 8, aptitude: true })
}
```
