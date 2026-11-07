# Arena search extensions

These fields extend `ournotes-deck.search-request/1`. Existing requests retain their scoring law and serialized result fields when the extensions are absent. Timing telemetry and wall-clock-limited incumbents are inherently execution-dependent.

```json
{
  "constraints": {"includeSnaps": [123, 456]},
  "context": {
    "powerSnapshot": {"eventIds": []},
    "gekisouRankPercents": [190, 220, 250],
    "gekisouRangeDiagnostics": true
  },
  "execution": {
    "kind": "live",
    "scoreId": 1004,
    "gekisou": true,
    "play": {"kind": "timingError", "sigmaMs": 30, "greatFraction": 0.01}
  }
}
```

This is a partial request: supply the ordinary format, scenario and metric fields too.

`constraints.includeSnaps` requires each distinct ID in every returned deck. Duplicate IDs are ignored. Unknown IDs, overlap with excludeSnaps, and combination with noSnaps are input errors. More than five distinct required Snaps makes the domain infeasible. Fixed decks and initial incumbents must also satisfy the constraint. The account recommendation v2 constraints wire supports the same field. Mandatory resources currently use physical enumeration with prefix feasibility checks: the existing top-K matching/class reductions cannot discard assignments safely under this constraint. Candidate strategy skips its unconstrained power-seed shortcut. The historical best-order diagnostic API rejects this field; use the physical recommendation API.

`context.gekisouRankPercents` is exactly three integer percentages in 0..=2147483647. They replace the master rank-1 *bonus* percentages, not the total score multiplier. This requires a solo Gekisou live and is incompatible with network confirmations. The simulator, recorded score programs and search's rank-factor schedules use the same percentages. Invalid or overflowing numerical bound domains continue to follow the engine's existing refusal rules.

`context.gekisouRangeDiagnostics: true` adds `gekisouRanges` to final deck results with a reported order. Timing-error play and an explicit rank override also enable these diagnostics automatically. The flag enables them for ordinary accuracy play without changing requests that use none of the extensions. The owned-snapshot NDJSON answer exposes them at `answer.result.results[i].gekisouRanges`. For expected aggregation, this is the already reported best order, **not averaged range counters**. Each row contains rangeIndex (zero based), mission, score (range end minus start), rankPercent (null if no bonus was awarded), combo, maxCombo, justCount and rawJust. `justCount` includes Just-bonus skills 13000/13002 and is the count used by the in-game ranking comparator. `rawJust` counts actual Just judgements after judgement conversion, without those bonus counts. Diagnostics are omitted for lottery-interval results, which have no single realized controller state, and when no best order is reported. Replaying the reported order for diagnostics adds work after the search's deadline.

`play.kind: timingError` requires finite positive sigmaMs, greatFraction in [0,1], and a solo Gekisou live. Greats use the existing deterministic accuracy distribution. Each remaining Just-eligible Perfect in a Just-mission range contributes

```
P(Just) = erf(beforeWindowMs / (sigmaMs * sqrt(2)))
```

The implementation uses the Abramowitz–Stegun 7.1.26 approximation (absolute error approximately 1.5e-7). It accumulates probability in note-stream order across the whole live, emits Just when the accumulator reaches one, and subtracts one. Other eligible notes remain Perfect. This deterministic realization is repeatable; it does not calculate an expectation over random judgement streams. Expected aggregation remains the mean over the 120 activation orders under this declared realization.

Effect 4004 uses the existing live skill updater and mutable window implementation: active owners add the rounded base-window increment, capped by the Miss window, then revert their own applied increments on expiry. Windows are read at the note's judge time before that frame's skill-update phase, following the existing simulator phase order. Dithering restarts for each order/deck. Ordinary accuracy and theoreticalBest keep their previous 4004 behavior. Other timing-window skills are not modeled by this new play law.

Expected timing-error search shares simulation prefixes and caches exact power-parameterized score programs: timing windows and dither mass are independent of total power. Leader variants are evaluated together to reuse these programs. Rotation splices remain disabled because they do not preserve timing-window/dither state. Maximum aggregation still uses direct order evaluation.

The score bound allows either Perfect or Just at every eligible Perfect in a Just range, independently of sigma and active windows. This covers even the widest possible assist window. Both count-trigger reachability and score masks use this relaxation; active 4004 rows remain part of Snap class identity. Unsupported conversions starting from a generated Just refuse the bound and fall back to enumeration. The bound can be loose; it does not promise completion within a time limit.

For branch-and-bound timing-error requests, a preliminary accuracy search uses the same Great fraction and Just fraction 0.5, with at most one quarter of the wall budget (capped at five seconds). Its returned decks are added to the initial candidates and evaluated under the actual timing-error law. No surrogate scores or bounds enter the final result. Once those initial evaluations complete, the returned incumbent cannot be worse than those re-scored candidates. This is not a guarantee against an accuracy search with a larger budget, and very short deadlines can expire before an initial evaluation completes. When bounds are available, a short local search then tries one-Snap 4004 replacements, restarting on improvements so multiple assists can be combined; these candidates are evaluated exactly under timing-error play. It uses at most a quarter of the remaining time before the normal warm start and traversal. LUCK-mission combinations are explicitly unsupported: the current lottery expectation evaluator assumes fixed judgements. Probability-dependent skills retain the engine's existing lottery-free handling on lives without LUCK ranges; any remaining random draw is rejected. Optional orderSample is not implemented.

Tests are synthetic. No inventory or private chart data is stored in the repository.
