import {readdirSync, readFileSync, writeFileSync} from 'fs'

import { Contract } from 'sevm';

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
  let funcs;
  try {
    funcs = new Contract(code).functions;
  } catch(e) {
    funcs = {};
  }

  if (mode == 'selectors') {
    return Object.keys(funcs)
  } else {
    return Object.fromEntries(selectors[fname].map((s) => {
      const fn = funcs[s];
      if (fn === undefined) {
        return [s, 'selnotfound'];
      } else {
        return [s, fn.constant ? 'view' : (fn.payable ? 'payable' : 'nonpayable')];
      }
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
