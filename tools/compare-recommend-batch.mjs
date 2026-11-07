// Read private inputs in memory only. No snapshots or answers are written to disk.
// Usage: node tools/compare-recommend-batch.mjs /absolute/path/to/calc [native-binary]
import {readFileSync} from 'node:fs';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {isDeepStrictEqual} from 'node:util';
import {performance} from 'node:perf_hooks';

const calc = resolve(process.argv[2] ?? '../calc');
const binary = resolve(process.argv[3] ?? 'target/release/recommend-batch');
const {loadData, createSolver, makeSnapshot, makeRequest, listCharts, table} =
  await import(pathToFileURL(join(calc, 'lib/engine.mjs')));
const {bytes, deck, datasetId, manifest} = loadData(join(calc, 'data/deck-data.json'));
const inventory = JSON.parse(readFileSync(join(calc, 'save/inventory.json'), 'utf8'));
const eventId = table(deck, 'MasterEvent').at(-1)._id;
const snapshot = makeSnapshot(inventory, deck, datasetId, eventId);
// A small real roster lets both engines exhaust the same search rather than compare
// machine-speed-dependent timeout incumbents. Keep five distinct characters, no Snaps.
const cards = new Map(table(deck, 'MasterMemberCard').map(c => [c._id, c]));
const characters = new Set();
snapshot.eligible.members = snapshot.eligible.members.filter(m => {
  const character = cards.get(m.id)._characterID;
  if (characters.size === 5 || characters.has(character)) return false;
  characters.add(character);
  return true;
});
if (characters.size !== 5) throw new Error('need five distinct owned characters');
snapshot.eligible.snaps = [];
const jobs = listCharts(deck, manifest).slice(0, 2).map((chart, i) => ({
  id: i, snapshot,
  request: {...makeRequest(deck, chart, 'score', eventId, {
    aggregation: 'expected', gekisou: true, greats: i, justFraction: 0.5,
  }), constraints: {noSnaps: true}},
}));
const solver = await createSolver(bytes, datasetId);
const startWasm = performance.now();
const wasm = jobs.map(j => ({id: j.id, answer: JSON.parse(solver.recommend(
  JSON.stringify(j.snapshot), JSON.stringify(j.request), () => {}, 1000,
))}));
const wasmMs = performance.now() - startWasm;
console.log(JSON.stringify({wasmMs, statuses: wasm.map(j => j.answer.status)}));
const startNative = performance.now();
const native = spawnSync(binary, ['--deck', join(calc, 'data/deck-data.json'), '--threads', '1'], {
  input: jobs.map(j => JSON.stringify(j)).join('\n') + '\n', encoding: 'utf8', maxBuffer: 32 * 1024 * 1024,
});
const nativeMs = performance.now() - startNative;
if (native.error) throw native.error;
if (native.status !== 0) throw new Error(native.stderr);
const answers = native.stdout.trim().split('\n').map(line => JSON.parse(line)).sort((a, b) => a.id - b.id);
// Timing telemetry is inherently different, even between two calls to the same engine.
// Keep the full exact comparison visible, and separately compare all non-time fields.
function withoutTimings(value) {
  if (Array.isArray(value)) return value.map(withoutTimings);
  if (!value || typeof value !== 'object') return value;
  return Object.fromEntries(Object.entries(value)
    .filter(([key]) => !key.endsWith("Ms"))
    .map(([key, v]) => [key, withoutTimings(v)]));
}
const exactEqual = isDeepStrictEqual(answers, wasm);
const nonTimingEqual = isDeepStrictEqual(withoutTimings(answers), withoutTimings(wasm));
console.log(JSON.stringify({jobs: jobs.length, threads: 1, wasmMs, nativeMs,
  nativeIncludesStartupAndDatasetParse: true, exactEqual, nonTimingEqual}));
if (!nonTimingEqual || wasm.some(j => j.answer.status !== 'ok')) process.exitCode = 1;
