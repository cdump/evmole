import json
import os
import sys

from Crypto.Hash import keccak

def sign(inp: bytes) -> str:
    return keccak.new(digest_bits=256, data=inp).digest()[:4].hex()

def join_inputs(inputs) -> str:
    if len(inputs) == 0:
        return ''
    n = ''
    for v in inputs:
        if v['type'].startswith('tuple'):
            n += '(' + join_inputs(v['components']) + ')' + v['type'][5:]
        else:
            n += v['type']
        n += ','
    return n[:-1]

def process(abi) -> dict[str,str]:
    ret = {}
    for x in abi:
        if x['type'] != 'function':
            continue
        args = join_inputs(x['inputs'])
        n = f'{x["name"]}({args})'
        sg = sign(n.encode('ascii'))
        ret[sg] = args
    return ret

if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE')
    sys.exit(1)


ret = {}
mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        r = process(d['abi'])
        ret[fname] = r if mode == 'arguments' else list(r.keys())

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
