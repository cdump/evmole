import argparse
import json
import os
import time

from evmole import contract_info, BlockType

parser = argparse.ArgumentParser()
parser.add_argument('mode', choices=['selectors', 'arguments', 'mutability', 'flow'])
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
        t0 = time.perf_counter_ns()
        if cfg.mode == 'selectors':
            info = contract_info(code, selectors=True)
        elif cfg.mode == 'arguments':
            info = contract_info(code, arguments=True)
        elif cfg.mode == 'mutability':
            info = contract_info(code, state_mutability=True)
        elif cfg.mode == 'flow':
            info = contract_info(code, control_flow_graph=True)
        else:
            raise Exception(f'Unknown mode {cfg.mode}')
        duration_us = int((time.perf_counter_ns() - t0) / 1000)

        if cfg.mode == 'selectors':
            r = [f.selector for f in info.functions]
        elif cfg.mode == 'arguments':
            by_sel = {f.selector: f.arguments for f in info.functions}
            r = {s: by_sel.get(s, 'notfound') for s in selectors[fname][1]}
        elif cfg.mode == 'mutability':
            by_sel = {f.selector: f.state_mutability for f in info.functions}
            r = {s: by_sel.get(s, 'notfound') for s in selectors[fname][1]}
        elif cfg.mode == 'flow':
            r = []
            for bl in info.control_flow_graph.blocks:
                match bl.btype:
                    case BlockType.Jump(to):
                        r.append((bl.start, to))
                    case BlockType.Jumpi(true_to, false_to):
                        r.append((bl.start, true_to))
                        r.append((bl.start, false_to))
                    case BlockType.DynamicJump(to):
                        for v in to:
                            if v.to is not None:
                                r.append((bl.start, v.to))
                    case BlockType.DynamicJumpi(true_to, false_to):
                        for v in true_to:
                            if v.to is not None:
                                r.append((bl.start, v.to))
                        r.append((bl.start, false_to))
                    case BlockType.Terminate:
                        pass
        else:
            raise Exception(f'Unknown mode {cfg.mode}')

        ret[fname] = [duration_us, r]

with open(cfg.output_file, 'w') as fh:
    json.dump(ret, fh)
