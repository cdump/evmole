import {readdirSync, readFileSync, writeFileSync} from 'fs'

import {functionArguments, functionSelectors} from './js/src/index.js'

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELCTORS_FILE]')
  process.exit(1)
}

let selectors = {}
const mode = argv[2];
const indir = argv[3];
const outfile = argv[4];

if (mode === 'arguments') {
  selectors = JSON.parse(readFileSync(argv[5]));
}

const res = Object.fromEntries(
  readdirSync(indir).map((file) => {
      const code = JSON.parse(readFileSync(`${indir}/${file}`))['code']
      let r = mode === 'arguments'
        ? Object.fromEntries(selectors[file].map((s) => [s, functionArguments(code, s)]))
        : functionSelectors(code);
      return [file, r];
    }
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
