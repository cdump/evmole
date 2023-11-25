import {readdirSync, readFileSync, writeFileSync} from 'fs'

import { whatsabi } from "@shazow/whatsabi";

const argv = process.argv;
if (argv.length != 4) {
  console.log('Usage: node main.js INPUT_DIR OUTPUT_FILE')
  process.exit(1)
}

const indir = argv[2];
const outfile = argv[3];

const res = Object.fromEntries(
  readdirSync(indir).map(
    file => [
      file,
      whatsabi.selectorsFromBytecode(JSON.parse(readFileSync(`${indir}/${file}`))['code']).map(x => x.slice(2)) // remove '0x' prefix
    ]
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
