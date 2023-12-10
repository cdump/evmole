import {readdirSync, readFileSync, writeFileSync} from 'fs'

import {functionSelectors} from './js/src/index.js'

const argv = process.argv;
if (argv.length < 5) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE')
  process.exit(1)
}

const mode = argv[2];
if (mode != 'selectors') {
  console.log('Only "selectors" mode supported, got ', mode)
  process.exit(1)
}
const indir = argv[3];
const outfile = argv[4];

const res = Object.fromEntries(
  readdirSync(indir).map(
    file => [
      file,
      functionSelectors(JSON.parse(readFileSync(`${indir}/${file}`))['code'])
    ]
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
