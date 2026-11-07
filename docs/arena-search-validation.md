# Arena extension verification

Changes are uncommitted on `local-arena`, based on coordinator commit `176c92a`. No lockfiles or calc files were changed. No dependencies were added, network fetches attempted, or private data copied.

## Checks

- `CARGO_BUILD_JOBS=2 cargo test --offline --release -p ournotes-sim -p ournotes-search -- --test-threads=1`: **987 passed, 0 failed, 6 ignored**, across 41 test binaries/doc-test groups. Release compilation took 7m40s on the occupied machine. Full local log: `/tmp/ournotes-arena-tests.log`.
- Targeted release arena tests: six passed (five new integration tests and the existing network battle/arena regression).
- Targeted release simulator timing-error tests: three passed (probability validation, dithering, 4004 stacking/cap/owner reversion).
- The account v2 includeSnaps wire unit test passed in the full suite.
- `cargo check --offline -p ournotes-search --tests`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo clippy --offline -p ournotes-sim -p ournotes-search --all-targets`: unavailable in default Cargo. Retrying with the installed stable toolchain on PATH failed to launch cargo-clippy with OS error 2.
- `cargo check --offline --manifest-path wasm/recommend/Cargo.toml`: blocked by separately locked `dlmalloc 0.2.14`, absent from cache. No WASM rebuild or dependency fetch.

## Local real-input smoke checks

The native batch executable accepted a reduced, five-character/no-Snap real inventory request with expected aggregation, timingError, rank overrides and range diagnostics: status ok, one result, three range rows, approximately 444 ms including startup and dataset parsing. Inputs stayed in memory; no private snapshots or answers were written.

The existing two-job comparison harness reports exactEqual=false and nonTimingEqual=false because platform, memory and phase telemetry also differ. Inspecting differing paths found only telemetry differences. Comparing complete answers after removing telemetry and elapsed-time fields gave **recommendationPayloadEqual=true** against the existing calc WASM on requests with no new fields. One run took about 87 ms in initialized WASM and 61 ms natively (native includes startup/data parsing); these small occupied-machine samples are not a benchmark.

## Limits

See [arena-search.md](arena-search.md) for the request fields and semantics. Diagnostics require `context.gekisouRangeDiagnostics: true` to preserve existing response fields, and are omitted without a single reported order or for lottery-interval results. Timing-error play is a deterministic dither with an erf approximation, restricted to solo Gekisou without LUCK ranges. Static-stream bounds and shortcuts are disabled for timing-error play; required Snaps also use physical enumeration, so large inventories may be slower. Other timing-window effect types are not added to the new play law. Historical best-order diagnostic search rejects includeSnaps; the physical recommendation APIs support it. Optional orderSample is not implemented.

## Changed files

Most existing test/fixture edits only initialize the new optional Rust struct fields to their previous defaults. Functional changes are in request parsing/domain enforcement, rank-factor setup, dynamic timing simulation and final-result diagnostics.

- `crates/ournotes-search/src/bin/ournotes-deck.rs`
- `crates/ournotes-search/src/domain.rs`
- `crates/ournotes-search/src/handler.rs`
- `crates/ournotes-search/src/handler/validation.rs`
- `crates/ournotes-search/src/recommendation/parse.rs`
- `crates/ournotes-search/src/recommendation/play.rs`
- `crates/ournotes-search/src/recommendation/tests.rs`
- `crates/ournotes-search/src/recommendation/wire.rs`
- `crates/ournotes-search/src/search/certified_engine.rs`
- `crates/ournotes-search/src/search/dispatch.rs`
- `crates/ournotes-search/src/search/expectation.rs`
- `crates/ournotes-search/src/search/leaf.rs`
- `crates/ournotes-search/src/search/mod.rs`
- `crates/ournotes-search/src/search/physical/engine.rs`
- `crates/ournotes-search/src/search/physical/entry.rs`
- `crates/ournotes-search/src/search/physical/enumeration.rs`
- `crates/ournotes-search/src/search/physical/maximum.rs`
- `crates/ournotes-search/src/search/physical/solve.rs`
- `crates/ournotes-search/src/search/session/tests.rs`
- `crates/ournotes-search/src/search/snaps/combo_triggers.rs`
- `crates/ournotes-search/src/search/snaps/gk_schedule.rs`
- `crates/ournotes-search/src/search/snaps/gk_schedule/timer_floor_tests.rs`
- `crates/ournotes-search/src/search/warm.rs`
- `crates/ournotes-search/src/types.rs`
- `crates/ournotes-search/tests/adapter_fixture_export.rs`
- `crates/ournotes-search/tests/chartstats.rs`
- `crates/ournotes-search/tests/fixtures/account_bounds.rs`
- `crates/ournotes-search/tests/fixtures/arena_extensions.rs`
- `crates/ournotes-search/tests/skip_points_bounds.rs`
- `crates/ournotes-sim/src/live/full/access.rs`
- `crates/ournotes-sim/src/live/full/apply.rs`
- `crates/ournotes-sim/src/live/full/build.rs`
- `crates/ournotes-sim/src/live/full/chart_cache_identity.rs`
- `crates/ournotes-sim/src/live/full/frames.rs`
- `crates/ournotes-sim/src/live/full/gaps_tests.rs`
- `crates/ournotes-sim/src/live/full/life_reads_tests.rs`
- `crates/ournotes-sim/src/live/full/luck.rs`
- `crates/ournotes-sim/src/live/full/luck_dp/state_identity_tests.rs`
- `crates/ournotes-sim/src/live/full/luck_dp/tests.rs`
- `crates/ournotes-sim/src/live/full/luck_exact.rs`
- `crates/ournotes-sim/src/live/full/luck_score_bounds/exact_orders_tests.rs`
- `crates/ournotes-sim/src/live/full/luck_score_bounds/exact_tests.rs`
- `crates/ournotes-sim/src/live/full/luck_score_bounds/owner_order_tests.rs`
- `crates/ournotes-sim/src/live/full/luck_score_bounds/probe_lifecycle_tests.rs`
- `crates/ournotes-sim/src/live/full/luck_score_bounds/tests.rs`
- `crates/ournotes-sim/src/live/full/mod.rs`
- `crates/ournotes-sim/src/live/full/score_program.rs`
- `crates/ournotes-sim/src/live/full/timing_error.rs`
- `crates/ournotes-sim/src/scenario.rs`
- `crates/ournotes-sim/tests/gekisou.rs`
- `crates/ournotes-sim/tests/gekisou_play.rs`
- `crates/ournotes-sim/tests/model_gaps.rs`
- `crates/ournotes-sim/tests/orders.rs`
- `crates/ournotes-sim/tests/raw_bridge.rs`
- `crates/ournotes-sim/tests/raw_just_fever.rs`
- `docs/arena-search-validation.md`
- `docs/arena-search.md`
