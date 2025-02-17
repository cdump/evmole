import { readdirSync, readFileSync, writeFileSync } from 'fs'
import { hrtime } from 'process'

import { contractInfo } from 'evmole'

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
  process.exit(1)
}

const mode = argv[2];
const indir = argv[3];
const outfile = argv[4];

const selectors = mode === 'mutability' || mode === 'arguments' ? JSON.parse(readFileSync(argv[5])) : {};

function timeit(fn) {
  const start_ts = hrtime.bigint();
  const r = fn();
  const duration_ms = Number((hrtime.bigint() - start_ts) / 1000000n);
  return [duration_ms, r]
}

function extract(code, mode, fname) {
  if (mode === 'selectors') {
    let [duration_ms, r] = timeit(() => contractInfo(code, {selectors: true}));
    return [duration_ms, r.functions.map((f) => f.selector)];
  } else if (mode === 'arguments') {
    let [duration_ms, r] = timeit(() => contractInfo(code, {arguments: true}));
    const by_sel = new Map(r.functions.map((f) => [f.selector, f.arguments]));
    return [duration_ms, Object.fromEntries(
      selectors[fname][1].map((s) => [s, by_sel.get(s) ?? 'notfound'])
    )];
  } else if (mode === 'mutability') {
    let [duration_ms, r] = timeit(() => contractInfo(code, {stateMutability: true}));
    const by_sel = new Map(r.functions.map((f) => [f.selector, f.stateMutability]));
    return [duration_ms, Object.fromEntries(
      selectors[fname][1].map((s) => [s, by_sel.get(s) ?? 'notfound'])
    )];
  } else if (mode === 'flow') {
    let [duration_ms, r] = timeit(() => contractInfo(code, {controlFlowGraph: true}));
    let ret = []
    for (const b of r.controlFlowGraph.blocks) {
      let bt = b.get('type');
      let start = b.get('start');
      let data = b.get('data');
      if (bt === 'Jump') {
        ret.push([start, data.to])
      } else if (bt === 'Jumpi') {
        ret.push([start, data.true_to])
        ret.push([start, data.false_to])
      } else if (bt === 'DynamicJump') {
        for (let v of data.to) {
          if(v.to) {
            ret.push([start, v.to])
          }
        }
      } else if (bt === 'DynamicJumpi') {
        for (let v of data.true_to) {
          if(v.to) {
            ret.push([start, v.to])
          }
        }
        ret.push([start, data.false_to])
      } else if (bt === 'Terminate') {
        // do nothing
      } else {
        throw `unknown block type ${bt}`;
      }
    }
    return [duration_ms, ret];
  } else {
    throw 'unsupported mode';
  }
}

const res = Object.fromEntries(
  readdirSync(indir).map(
    file => [
      file,
      extract(JSON.parse(readFileSync(`${indir}/${file}`))['code'], mode, file)
    ]
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
