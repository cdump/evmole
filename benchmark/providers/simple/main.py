import json
import os
import sys

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

def extract_arguments(code: bytes, selector: bytes) -> str:
    return ''


if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
    sys.exit(1)

ret = {}
mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

selectors = {}
if mode == 'arguments':
    selectors_file = sys.argv[4]
    with open(selectors_file, 'r') as fh:
        selectors = json.load(fh)

for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        code = bytes.fromhex(d['code'][2:])
        if mode == 'arguments':
            r = {s: extract_arguments(code, bytes.fromhex(s)) for s in selectors[fname]}
        else:
            r = extract_selectors(code)
        ret[fname] = r

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
