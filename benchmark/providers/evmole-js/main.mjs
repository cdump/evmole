import {readdirSync, readFileSync, writeFileSync} from 'fs'
import {parseArgs} from 'util'

import {functionArguments, functionSelectors, functionStateMutability} from 'evmole'

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

const selectors = mode === 'selectors' ? {} : JSON.parse(readFileSync(cfg_rest[0]));

const res = Object.fromEntries(
  readdirSync(indir)
    .filter((file) => cfg['filter-filename'] === undefined || file.includes(cfg['filter-filename']))
    .map((file) => {
      const code = JSON.parse(readFileSync(`${indir}/${file}`))['code']
      if (mode === 'selectors') {
        return [file, functionSelectors(code)];
      } else {
        const fsel = cfg['filter-selector'] === undefined ? selectors[file] : [cfg['filter-selector']];
        if (mode === 'arguments') {
          return [file, Object.fromEntries(fsel.map((s) => [s, functionArguments(code, s)]))];
        } else {
          return [file, Object.fromEntries(fsel.map((s) => [s, functionStateMutability(code, s)]))];
        }
      }
    }
  )
);
writeFileSync(outfile, JSON.stringify(res), 'utf8');
