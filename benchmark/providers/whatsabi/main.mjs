import {readdirSync, readFileSync, writeFileSync} from 'fs'

import { whatsabi } from "@shazow/whatsabi";

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
  process.exit(1)
}

const mode = argv[2];
if (mode != 'selectors' && mode != 'mutability') {
  console.log('Only "selectors" and "mutability" modes are supported, got ', mode)
  process.exit(1)
}
const indir = argv[3];
const outfile = argv[4];

const selectors = mode === 'selectors' ? {} : JSON.parse(readFileSync(argv[5]));

function extract(code, mode, fname) {
  if (mode == 'selectors') {
    return whatsabi.selectorsFromBytecode(code).map(x => x.slice(2)); // remove '0x' prefix
  } else { // mutability
    const abi = whatsabi.abiFromBytecode(code);
    const smut = Object.fromEntries(abi.filter((v) => v.type == 'function').map((v) => [v.selector, v.stateMutability]));
    return Object.fromEntries(selectors[fname].map((s) => {
      return [s, smut[`0x${s}`] || 'selnotfound'];
    }));
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
