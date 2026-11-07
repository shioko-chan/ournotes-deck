// Local real-data benchmark. Private snapshots/answers stay in memory.
// node tools/benchmark-arena.mjs [calc path] [time limit ms] [fixed|full] [accuracy|timingError] [arena JSON basename]
// fixed restricts the eligible roster to five members/five Snaps; bindings and leaders still vary.
// full uses the same band (13 members in the reported inventory) and all owned Snaps.
// ARENA_COMPARE_SEED=1 separately runs/re-scores the accuracy surrogate for comparison.
import {readFileSync} from 'node:fs';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
const calc = resolve(process.argv[2] ?? '../calc');
const limit = Number(process.argv[3] ?? 60000);
const mode = process.argv[4] ?? 'fixed';
const kind = process.argv[5] ?? 'timingError';
const {loadData, makeSnapshot, makeRequest, listCharts, table} = await import(pathToFileURL(join(calc, 'lib/engine.mjs')));
const path = join(calc, 'data/arena', process.argv[6] ?? 'deck-league3-regulation70-just.json');
const {deck, datasetId, manifest} = loadData(path);
const inventory = JSON.parse(readFileSync(join(calc, 'save/inventory.json'), 'utf8'));
const eventId = table(deck, 'MasterEvent').at(-1)._id;
const snapshot = makeSnapshot(inventory, deck, datasetId, eventId);
const cards = new Map(table(deck, 'MasterMemberCard').map(c => [c._id,c]));
const characters = new Map(table(deck, 'MasterCharacter').map(c => [c._id,c]));
const band = characters.get(cards.get(36)._characterID)._bandID;
snapshot.eligible.members = snapshot.eligible.members.filter(m => characters.get(cards.get(m.id)._characterID)._bandID === band);
if (mode === 'fixed') {
  snapshot.eligible.members = snapshot.eligible.members.filter(m => [36,39,63,61,62].includes(m.id));
  snapshot.eligible.snaps = snapshot.eligible.snaps.filter(s => [64,63,56,39,62].includes(s.id));
}
const chart = listCharts(deck, manifest).find(c => c.musicId === 100050 && c.difficulty.toUpperCase() === 'EXPERT');
const request = makeRequest(deck, chart, 'score', eventId, {aggregation:'expected', gekisou:true, greats:11, justFraction:0.5, timeLimitMs:limit});
if (kind === 'timingError') request.execution.play = {kind, sigmaMs:29, greatFraction:11/chart.noteCount};
request.context.gekisouRangeDiagnostics = true;
request.context.gekisouRankPercents = [190,190,190];
function runJob(request) {
  const start = performance.now();
  const run = spawnSync(resolve(process.env.ARENA_BINARY ?? 'target/release/recommend-batch'), ['--deck', path, '--threads','1'], {
    input: JSON.stringify({id:1,snapshot,request})+'\n', encoding:'utf8', maxBuffer:32*1024*1024,
  });
  if (run.error) throw run.error;
  if (run.status) throw new Error(run.stderr);
  return {answer:JSON.parse(run.stdout).answer, wallMs:performance.now()-start};
}
const {answer,wallMs} = runJob(request);
const result = answer.result;
let accuracySeed;
if (kind === 'timingError' && process.env.ARENA_COMPARE_SEED === '1') {
  const surrogate = structuredClone(request);
  surrogate.execution.play = {kind:'accuracy',greatFraction:11/chart.noteCount,justFraction:0.5};
  surrogate.limits.timeLimitMs = Math.min(5000, Math.floor(limit/4));
  const baseline = runJob(surrogate);
  const seed = baseline.answer.result?.results[0];
  if (seed) {
    const rescore = structuredClone(request);
    rescore.strategy = {kind:'exhaustive'};
    // The first and only evaluated candidate is this physical binding; no search substitution.
    rescore.initialDecks = [{members:seed.members,snaps:seed.snaps}];
    rescore.limits = {timeLimitMs:60000,maxCandidates:1};
    const rescored = runJob(rescore);
    const seededScore = rescored.answer.result?.results[0]?.expectedScore;
    const actual = result?.results[0]?.expectedScore;
    accuracySeed = {wallMs:baseline.wallMs, status:baseline.answer.status,
      rescoreStatus:rescored.answer.status,rescoreErrors:rescored.answer.errors,
      score:seed.expectedScore, rescoredScore:seededScore,
      incumbentAtLeastSeed: actual && seededScore &&
        BigInt(actual.numerator)*BigInt(seededScore.denominator) >= BigInt(seededScore.numerator)*BigInt(actual.denominator)};
  }
}
console.log(JSON.stringify({kind,mode,limit,members:snapshot.eligible.members.length,snapCount:snapshot.eligible.snaps.length,
 wallMs,status:answer.status,errors:answer.errors,accuracySeed,
 completion:result?.completion,score:result?.results[0]?.expectedScore,
 snaps:result?.results[0]?.snaps,ranges:result?.results[0]?.gekisouRanges,
 telemetry:result?.telemetry}));
