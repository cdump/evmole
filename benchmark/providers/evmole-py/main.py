import argparse
import json
import os
import time

from evmole import function_arguments, function_selectors, function_state_mutability

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
            r = function_selectors(code)
        elif cfg.mode == 'arguments':
            fsel = selectors[fname][1]
            r = {s: function_arguments(code, s) for s in fsel}
        elif cfg.mode == 'mutability':
            fsel = selectors[fname][1]
            r = {s: function_state_mutability(code, s) for s in fsel}
        else:
            raise Exception(f'Unknown mode {cfg.mode}')
        duration_ms = int((time.perf_counter() - t0) * 1000)
        ret[fname] = [duration_ms, r]

with open(cfg.output_file, 'w') as fh:
    json.dump(ret, fh)
