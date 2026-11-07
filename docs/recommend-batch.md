# Native batch recommendations

Build from the engine workspace with cached dependencies:

```sh
cargo build --offline --release -p ournotes-search --bin recommend-batch
./target/release/recommend-batch --deck ../calc/data/deck-data.json --threads 4 < jobs.ndjson > answers.ndjson
```

Each stdin line is a JSON envelope:

```json
{"id":"job-1","snapshot":{"format":"ournotes.owned-snapshot/1","...":"snapshot fields"},"request":{"format":"ournotes-deck.search-request/1","...":"request fields"}}
```

Each stdout line is `{"id":"job-1","answer":{...}}`. The ID may be any JSON value. The answer is the serialized result of `engine::recommend_snapshot`, exactly the facade called by `DeckSolver.recommend` for owned snapshots. The CLI does not emit progress answers. Lines are emitted in completion order; use IDs to correlate them. The worker count defaults to available parallelism and must be positive. One immutable parsed dataset is shared by all workers, with a bounded input queue. Output is flushed after every answer.

Invalid snapshots and requests receive the engine's structured error answer. Malformed NDJSON envelopes, input/output errors and invalid arguments cause a nonzero exit with a diagnostic on stderr; already completed jobs may have emitted answers. Blank lines are malformed envelopes. This transport accepts owned snapshots, not the separate account recommendation envelope.

## Native/WASM comparison

```sh
node tools/compare-recommend-batch.mjs ../calc ./target/release/recommend-batch
```

The script imports calc's existing adapters and WASM module, reads local inventory only into memory, selects five distinct owned characters with no Snaps, and runs two chart requests with Gekisou, accuracy play, and expected aggregation. It writes no inventory, snapshot or answer files. Both searches run without a time limit so they can exhaust the same small domain. It reports full JSON equality and, separately, equality after removing timing fields ending in `Ms`. A mismatch in the latter comparison or a non-ok WASM answer fails the command. Native timing includes process startup and dataset parsing; WASM timing excludes initialization. The native run uses one worker for this comparison.

Elapsed-time telemetry inherently prevents byte-identical answers across executions. Time-limited requests can also visit different numbers of candidates and return different decks; sharing the scoring code cannot guarantee equal timeout answers.

## Local verification status

On this checkout, offline build/test resolution is blocked by the locked `cpufeatures 0.3.1` dependency (only 0.3.0 and 0.2.17 are cached). An isolated source-copy attempt to resolve cached versions also found missing `hybrid-array`. No dependencies or lockfiles were changed and no network fetch was attempted. Clippy is absent from the default Cargo commands; the installed stable toolchain's cargo-clippy executable fails to launch. The installed toolchain has no wasm32 target or wasm-pack.

The comparison script ran both WASM jobs successfully in 255 ms, but native compilation and thus native equality/timing remain unverified. CLI unit tests cover envelope validation and exact forwarding of structured snapshot errors; execution is blocked by dependency resolution. The requested includeSnaps, rank override, range diagnostics, timing-error play and optional order sampling are not implemented in this change.
