import {readdirSync, readFileSync, writeFileSync} from 'fs'

import { Contract } from 'sevm';

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
  process.exit(1)
}

const mode = argv[2];
if (mode != 'selectors') {
  console.log('Only "selectors" mode supported, got ', mode)
  process.exit(1)
}
const indir = argv[3];
const outfile = argv[4];

function extract(code) {
  try {
    return Object.keys(new Contract(code).functions)
  } catch(e) {
    return []
  }
}

const res = Object.fromEntries(
  readdirSync(indir).map(
    file => [
      file,
      extract(JSON.parse(readFileSync(`${indir}/${file}`))['code'])
    ]
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
