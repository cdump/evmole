import argparse
import json
import os
import time

from evmole import contract_info

parser = argparse.ArgumentParser()
parser.add_argument('mode', choices=['selectors', 'arguments', 'mutability'])
parser.add_argument('input_dir')
parser.add_argument('output_file')
parser.add_argument('selectors_file', nargs='*')
cfg = parser.parse_args()

selectors = {}
if cfg.mode != 'selectors':
    with open(cfg.selectors_file[0], 'r') as fh:
        selectors = json.load(fh)

ret = {}
for fname in os.listdir(cfg.input_dir):
    with open(f'{cfg.input_dir}/{fname}', 'r') as fh:
        d = json.load(fh)
        code = d['code']
        t0 = time.perf_counter()
        if cfg.mode == 'selectors':
            r = contract_info(code, selectors=True)
        elif cfg.mode == 'arguments':
            r = contract_info(code, arguments=True)
        elif cfg.mode == 'mutability':
            r = contract_info(code, state_mutability=True)
        else:
            raise Exception(f'Unknown mode {cfg.mode}')
        duration_ms = int((time.perf_counter() - t0) * 1000)

        if cfg.mode == 'selectors':
            r = [f.selector for f in r.functions]
        elif cfg.mode == 'arguments':
            by_sel = {f.selector: f.arguments for f in r.functions}
            r = {s: by_sel.get(s, 'notfound') for s in selectors[fname][1]}
        elif cfg.mode == 'mutability':
            by_sel = {f.selector: f.state_mutability for f in r.functions}
            r = {s: by_sel.get(s, 'notfound') for s in selectors[fname][1]}
        else:
            raise Exception(f'Unknown mode {cfg.mode}')

        ret[fname] = [duration_ms, r]

with open(cfg.output_file, 'w') as fh:
    json.dump(ret, fh)
