import json
import time
import os
import sys

from evm_cfg_builder import CFG


def extract_cfg(code_hex: str):
    start_ts = time.monotonic()
    result = []
    try:
        cfg = CFG(code_hex)
    except Exception as e:
        print(e)
        duration_us = int(time.monotonic() - start_ts)
        return [duration_us, []]

    duration_us = int(time.monotonic() - start_ts)
    for x in cfg.basic_blocks:
        assert all(ins.mnemonic != 'JUMPDEST' for ins in x.instructions[1:]), x.instructions
    result = [(basic_block.start.pc, out.start.pc) for basic_block in cfg.basic_blocks for out in basic_block.all_outgoing_basic_blocks]

    return [duration_us, sorted(result)]


if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
    sys.exit(1)

ret = {}
mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

assert mode == 'flow'

for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        ret[fname] = extract_cfg(d['code'])

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
