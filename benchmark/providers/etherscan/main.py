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

def process(abi) -> list[str]:
    ret = {}
    for x in abi:
        if x['type'] != 'function':
            continue
        n = x['name'] + '(' + join_inputs(x['inputs']) + ')'
        sg = sign(n.encode('ascii'))
        ret[sg] = n
    return list(ret.keys())

if len(sys.argv) != 3:
    print('Usage: python3 main.py INPUT_DIR OUTPUT_FILE')
    sys.exit(1)


ret = {}
indir = sys.argv[1]
outfile = sys.argv[2]
for fname in os.listdir(indir):
    with open(f'{indir}/{fname}', 'r') as fh:
        d = json.load(fh)
        ret[fname] = process(d['abi'])

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
