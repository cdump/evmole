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
