import json
import os
import sys
import time


def extract_selectors(code: bytes) -> list[str]:
    ret = []
    for i in range(len(code) - 5):
        # PUSH3/PUSH4
        if (code[i] == 0x62 or code[i] == 0x63):
            off = code[i] - 0x62

            # EQ or (DUP2 + EQ)
            if (code[i+off+4] == 0x14) or (code[i+off+4] == 0x81 and code[i+off+5] == 0x14):
                ret.append(code[i+1:i+4+off])

    return [s.hex().zfill(8) for s in ret]

if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
    sys.exit(1)

ret = {}
mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

selectors = {}
if mode != 'selectors':
    selectors_file = sys.argv[4]
    with open(selectors_file, 'r') as fh:
        selectors = json.load(fh)

for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        code = bytes.fromhex(d['code'][2:])
        t0 = time.perf_counter()
        if mode == 'arguments':
            r = {s: '' for s in selectors[fname][1]}
        elif mode == 'mutability':
            r = {s: 'nonpayable' for s in selectors[fname][1]}
        elif mode == 'selectors':
            r = extract_selectors(code)
        else:
            raise Exception(f'Unknown mode {mode}')
        duration_us = int(time.perf_counter() - t0)
        ret[fname] = [duration_us, r]

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
