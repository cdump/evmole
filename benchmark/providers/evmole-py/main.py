import json
import os
import sys

from evmole import function_selectors

if len(sys.argv) != 3:
    print('Usage: python3 main.py INPUT_DIR OUTPUT_FILE')
    sys.exit(1)


ret = {}
indir = sys.argv[1]
outfile = sys.argv[2]
for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        ret[fname] = function_selectors(bytes.fromhex(d['code'][2:]))

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
