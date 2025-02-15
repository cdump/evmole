import { readdirSync, readFileSync, writeFileSync } from 'fs'
import { hrtime } from 'process'

import { Contract } from 'sevm';

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
  process.exit(1)
}

const mode = argv[2];
const indir = argv[3];
const outfile = argv[4];

const selectors = mode === 'mutability' ? JSON.parse(readFileSync(argv[5])) : {};

function timeit(fn) {
  const start_ts = hrtime.bigint();
  const r = fn();
  const duration_ms = Number((hrtime.bigint() - start_ts) / 1000000n);
  return [duration_ms, r]
}

function extract(code, mode, fname) {
  const [duration_ms, contract] = timeit(() => {
    try {
      return new Contract(code);
    } catch (e) {
      // console.log(e);
    }
  });
  if (mode === 'selectors') {
    return [duration_ms, Object.keys(contract ? contract.functions : {})]
  } else if (mode === 'mutability') {
    return [duration_ms, Object.fromEntries(selectors[fname][1].map((s) => {
      const fn = contract ? contract.functions[s] : undefined;
      if (fn === undefined) {
        return [s, 'selnotfound'];
      } else {
        return [s, fn.constant ? 'view' : (fn.payable ? 'payable' : 'nonpayable')];
      }
    }))];
  } else if (mode === 'flow') {
    let res = new Map();
    const add = (from, to) => {
      res.set(`${from}|${to}`, [from, to]);
    };
    for (const [pc, block] of (contract ? contract.blocks : [])) {
      for (const {opcode} of block.opcodes.slice(1)) { // skip first
        if (opcode.opcode === 91) {
          throw 'JUMPDEST inside block';
        }
      }
      for (const state of block.states) {
        switch (state.last?.name) {
          case 'Jumpi':
            add(pc, state.last.destBranch.pc);
            add(pc, state.last.fallBranch.pc);
            break;
          case 'SigCase':
            add(pc, state.last.fallBranch.pc);
            break;
          case 'Jump':
            add(pc, state.last.destBranch.pc);
            break;
          case 'JumpDest':
            add(pc, state.last.fallBranch.pc);
            break;
          default:
        }
      }
    }
    return [duration_ms, Array.from(res.values())];
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
