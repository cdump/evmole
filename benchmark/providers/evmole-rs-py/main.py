import argparse
import json
import os

from evmole import function_selectors, function_arguments


parser = argparse.ArgumentParser()
parser.add_argument('mode', choices=['selectors', 'arguments'])
parser.add_argument('input_dir')
parser.add_argument('output_file')
parser.add_argument('selectors_file', nargs='*')
parser.add_argument('--filter-filename', required=False)
parser.add_argument('--filter-selector', required=False)
cfg = parser.parse_args()

selectors = {}
if cfg.mode == 'arguments':
    with open(cfg.selectors_file[0], 'r') as fh:
        selectors = json.load(fh)

ret = {}
for fname in os.listdir(cfg.input_dir):
    if cfg.filter_filename is not None and cfg.filter_filename not in fname:
        continue

    with open(f'{cfg.input_dir}/{fname}', 'r') as fh:
        d = json.load(fh)
        code = d['code']
        if cfg.mode == 'arguments':
            fsel = selectors[fname] if cfg.filter_selector is None else [cfg.filter_selector]
            r = {s: function_arguments(code, s) for s in fsel}
        else:
            r = function_selectors(code)
        ret[fname] = r

with open(cfg.output_file, 'w') as fh:
    json.dump(ret, fh)
