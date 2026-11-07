// Private inventory is read at runtime only; no snapshots or answers are written.
// node tools/benchmark-skill-shapes.mjs [calc path] [limit ms] [open]
// Output pairs: A without/with member; then B items, points, score without/with snap.
// Optional open adds the same B pairs with all eligible members and snaps.
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
const calc = resolve(process.argv[2] ?? '../calc');
const limit = Number(process.argv[3] ?? 8000);
const {loadData, loadInventory, makeSnapshot, makeRequest, listCharts, table} = await import(pathToFileURL(join(calc, 'lib/engine.mjs')));
const path = join(calc, 'data/deck-data.json');
const {deck, datasetId, manifest} = loadData(path);
const inventory = loadInventory(join(calc, 'save/inventory.json'), deck, {cap:false});
const base = makeSnapshot(inventory, deck, datasetId, 4);
if (!base.eligible.members.some(m => m.id === 72)) throw new Error('Benchmark requires the release-controlled member');
const charts = listCharts(deck, manifest);
const normal = charts.find(c => c.difficulty === 'EXPERT');
const challengeIds = new Set(table(deck, 'MasterChallengeMusic').filter(c => c._eventId === 4).map(c => c._liveMusicId));
const challenge = charts.find(c => c.difficulty === 'EXPERT' && challengeIds.has(c.musicId));
function run(snapshot, request) {
 const start = performance.now();
 const p = spawnSync(resolve('target/release/recommend-batch'), ['--deck',path,'--threads','1'], {input:JSON.stringify({id:0,snapshot,request})+'\n',encoding:'utf8',maxBuffer:32*1024*1024});
 if (p.error || p.status) throw new Error('Native benchmark failed');
 const a = JSON.parse(p.stdout).answer;
 if (a.status !== 'ok') throw new Error('Native request rejected');
 console.log(JSON.stringify({wallMs:Math.round(performance.now()-start),power:a.result.results[0]?.power,exitReason:a.result.exitReason}));
}
for (const include of [false,true]) {
 const s = structuredClone(base); s.eligible.snaps=[];
 s.eligible.members=s.eligible.members.filter(m=>include || m.id!==72);
 run(s,makeRequest(deck,normal,'score',4,{aggregation:'maximum',greats:11,timeLimitMs:limit}));
}
const cards = new Map(table(deck,'MasterMemberCard').map(c=>[c._id,c]));
const members=[]; const chars=new Set();
for(const m of base.eligible.members) {const ch=cards.get(m.id)._characterID;if(!chars.has(ch)&&members.length<5){members.push(m);chars.add(ch);}}
const ordinary=base.eligible.snaps.filter(s=>s.id!==76).slice(0,5);
const special=base.eligible.snaps.find(s=>s.id===76);
if(members.length!==5||ordinary.length!==5||!special) throw new Error('Benchmark requires five members and six suitable snaps');
for (const goal of ['challenge-items','challenge-pt','score']) for(const include of [false,true]) {
 const s=structuredClone(base);s.eligible.members=members;s.eligible.snaps=include?[...ordinary.slice(0,4),special]:ordinary;
 const r=makeRequest(deck,challenge,goal,4,{greats:11,timeLimitMs:limit});
 if(goal==='score')r.scenario=makeRequest(deck,challenge,'challenge-score',4).scenario;
 run(s,r);
}

if (process.argv[4] === 'open') for (const goal of ['challenge-items','challenge-pt','score']) for(const include of [false,true]) {
 const s=structuredClone(base);
 s.eligible.snaps=s.eligible.snaps.filter(s=>include || s.id!==76);
 const r=makeRequest(deck,challenge,goal,4,{greats:11,timeLimitMs:limit});
 if(goal==='score')r.scenario=makeRequest(deck,challenge,'challenge-score',4).scenario;
 run(s,r);
}
