import { readdirSync, readFileSync, writeFileSync } from 'fs'
import { hrtime } from 'process'

import { whatsabi } from "@shazow/whatsabi";

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
  if (mode === 'selectors') {
    const [duration_ms, r] = timeit(() => whatsabi.selectorsFromBytecode(code))
    return [duration_ms, r.map(x => x.slice(2))]; // remove '0x' prefix
  } else if (mode === 'mutability') {
    const [duration_ms, abi] = timeit(() => whatsabi.abiFromBytecode(code));
    const smut = Object.fromEntries(abi.filter((v) => v.type == 'function').map((v) => [v.selector, v.stateMutability]));
    return [duration_ms, Object.fromEntries(selectors[fname][1].map((s) => [s, smut[`0x${s}`] || 'selnotfound']))];
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
