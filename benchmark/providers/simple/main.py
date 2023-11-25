import json
import os
import sys

def process(code: bytes) -> list[str]:
    ret = []
    for i in range(len(code) - 5):
        # PUSH2/PUSH3
        if (code[i] == 0x62 or code[i] == 0x63):
            off = code[i] - 0x62

            # EQ or (DUP2 + EQ)
            if (code[i+off+4] == 0x14) or (code[i+off+4] == 0x81 and code[i+off+5] == 0x14):
                ret.append(code[i+1:i+4+off])

    return [s.hex().zfill(8) for s in ret]


if len(sys.argv) != 3:
    print('Usage: python3 main.py INPUT_DIR OUTPUT_FILE')
    sys.exit(1)


ret = {}
indir = sys.argv[1]
outfile = sys.argv[2]
for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        ret[fname] = process(bytes.fromhex(d['code'][2:]))

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
