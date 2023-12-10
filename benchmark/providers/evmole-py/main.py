import json
import os
import sys

from evmole import function_selectors, function_arguments

if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
    sys.exit(1)


ret = {}
mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

if mode == 'arguments':
    selectors_file = sys.argv[4]
    with open(selectors_file, 'r') as fh:
        selectors = json.load(fh)

for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        code = d['code']
        if mode == 'arguments':
            r = {s: function_arguments(code, s) for s in selectors[fname]}
        else:
            r = function_selectors(code)
        ret[fname] = r

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
