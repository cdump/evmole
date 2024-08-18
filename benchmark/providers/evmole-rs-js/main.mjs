import {readdirSync, readFileSync, writeFileSync} from 'fs'
import {parseArgs} from 'util'

import {functionArguments, functionSelectors} from './js/src/index.js'

const {
  values: cfg,
  positionals: cfg_positionals,
} = parseArgs({
  options: {
    'filter-filename': {
      type: 'string',
    },
    'filter-selector': {
      type: 'string',
    },
  },
  allowPositionals: true
});

if (cfg_positionals.length < 3) {
  console.log('Usage: node main.js MODE INPUT_DIR OUTPUT_FILE [SELCTORS_FILE]')
  process.exit(1)
}

const [mode, indir, outfile, ...cfg_rest] = cfg_positionals;

const selectors = mode === 'arguments' ? JSON.parse(readFileSync(cfg_rest[0])) : {};

const res = Object.fromEntries(
  readdirSync(indir)
    .filter((file) => cfg['filter-filename'] === undefined || file.includes(cfg['filter-filename']))
    .map((file) => {
      const code = JSON.parse(readFileSync(`${indir}/${file}`))['code']
      const fsel = cfg['filter-selector'] === undefined ? selectors[file] : [cfg['filter-selector']];
      let r = mode === 'arguments'
        ? Object.fromEntries(fsel.map((s) => [s, functionArguments(code, s)]))
        : functionSelectors(code);
      return [file, r];
    }
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
