# Arena follow-up validation

The owned-snapshot serializer already forwarded the optional diagnostic field. The prior implementation required the diagnostics flag. Timing-error play and explicit rank overrides now enable diagnostics automatically; the explicit flag still enables them for ordinary play. Requests without these extensions retain their result fields.

Expected timing-error evaluation now shares order prefixes, records power-parameterized programs, groups leader variants for cache locality, and uses a conservative Perfect/Just reachability envelope. Timing assists remain distinct in Snap class identities. An accuracy seed is re-scored under timing-error play, followed by an exact local search over timing-assist replacements. These heuristics affect traversal only, not the scoring law or proof requirements.

## Checks

- `CARGO_BUILD_JOBS=2 cargo test --offline --release -p ournotes-sim -p ournotes-search -- --test-threads=1`: 990 passed, 0 failed, 6 ignored across 41 suites.
- After the final warm-start traversal adjustment: `cargo test --offline --release -p ournotes-search --test adapter_fixture_export arena_ -- --test-threads=1`: 8 passed.
- `cargo build --offline --release -p ournotes-search --bin recommend-batch`: passed.
- `cargo fmt --all -- --check`, `git diff --check`, `node --check tools/benchmark-arena.mjs`: passed.
- Clippy could not launch. Default Cargo reports no Clippy command; prepending the installed stable toolchain attempts `cargo-clippy` but fails with OS error 2 (never executed).
- Two ordinary requests matched the existing WASM answer after excluding telemetry and all `*Ms` fields. Raw JSON equality is false because of those execution/platform fields. The comparison harness was transformed in memory for this check; its repository source was not changed.

## Real-data reproduction

Private snapshots and answers stay in memory. The benchmark prints only result summaries and telemetry, and writes no data files. `fixed` means a restricted five-member/five-Snap roster; the search still chooses bindings and leaders. `full` selects the same band and all eligible Snaps. Accuracy uses Just fraction 0.5; timing-error uses sigma 29 ms. Both use 11 Greats, expected aggregation, and rank bonus percentages 190/190/190.

```sh
node tools/benchmark-arena.mjs ../calc 60000 fixed accuracy
ARENA_COMPARE_SEED=1 node tools/benchmark-arena.mjs ../calc 60000 fixed timingError
ARENA_COMPARE_SEED=1 node tools/benchmark-arena.mjs ../calc 60000 full timingError
# Optional final argument: deck-league3-regulation69.json (default: regulation70-just)
```

`ARENA_COMPARE_SEED=1` independently runs the accuracy surrogate, then re-scores its exact physical binding under timing-error play with a one-candidate exhaustive request. The output reports whether the final incumbent is at least that score. Timings are wall time including native startup and dataset parsing.

## Remaining limitation

The requested 2–3× accuracy proof-time target is not met. The widest-window bound is conservative but loose; the restricted roster still timed out in a 180-second run. The accuracy seed and assist search improve early incumbents, but do not certify them or guarantee parity with a longer-running accuracy search. Short budgets can expire before seed evaluation completes. Maximum aggregation still uses its prior direct timing-error order path.

## Files changed

- `crates/ournotes-search/src/engine.rs`
- `crates/ournotes-search/src/handler.rs`
- `crates/ournotes-search/src/search/dispatch.rs`
- `crates/ournotes-search/src/search/leaf.rs`
- `crates/ournotes-search/src/search/physical/engine.rs`
- `crates/ournotes-search/src/search/snaps/combo_triggers.rs`
- `crates/ournotes-search/src/search/snaps/envelope_data.rs`
- `crates/ournotes-search/src/search/snaps/fine_view.rs`
- `crates/ournotes-search/src/search/snaps/gk_windows.rs`
- `crates/ournotes-search/src/search/snaps/order_sums.rs`
- `crates/ournotes-search/src/search/snaps/raw.rs`
- `crates/ournotes-search/src/search/snaps/rows.rs`
- `crates/ournotes-search/src/search/snaps/snap_live_build.rs`
- `crates/ournotes-search/src/search/warm.rs`
- `crates/ournotes-search/src/types.rs`
- `crates/ournotes-search/tests/fixtures/arena_extensions.rs`
- `crates/ournotes-sim/src/live/full/score_program.rs`
- `crates/ournotes-sim/src/scenario.rs`
- `docs/arena-search.md`
- `tools/benchmark-arena.mjs`
- `docs/arena-followup-validation.md`
