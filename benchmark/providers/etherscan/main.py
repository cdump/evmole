import json
import os
import re
import sys
import time

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

def process_storage_mapping(types, k, v) -> str:
    kt = types[k]
    vt = types[v]
    if isinstance(vt, str):
        return f'mapping({kt} => {vt})'

    if isinstance(vt, dict):
        assert len(vt) == 1
        val = process_storage_mapping(types, *list(vt.items())[0])
        return f'mapping({kt} => {val})'

    if isinstance(vt, tuple):
        if len(vt) == 1:
            # struct with only 1 field:
            return process_storage_mapping(types, k, vt[0]['type'])
        else:
            return f'mapping({kt} => struct_{len(vt)}_fields)'

    if isinstance(vt, list):
        val = process_storage_dynarray(types, types[vt[0]])
        return f'mapping({kt} => {val})'

    raise Exception(f'Unsupported map type {kt} / {vt}')

def process_storage_dynarray(types, base) -> str:
    if isinstance(base, str):
        return f'{base}[]'
    if isinstance(base, tuple):
        if len(base) == 1:
            return process_storage_dynarray(types, base[0]) + '[]'
        else:
            return f'struct_{len(base)}_fields[]'

    if isinstance(base, list):
        return process_storage_dynarray(types, types[base[0]]) + '[]'

    raise Exception(f'Unsupported dynamic array base type {base}')

def process_storage_value(types, base_slot: int, offset, value) -> dict[str, str]:
    key = f'{base_slot:064x}_{offset}'
    if isinstance(value, str):
        return {key: value}
    elif isinstance(value, tuple):
        assert offset == 0
        ret: dict[str, str] = {}
        for y in value:
            r = process_storage_value(types, base_slot + int(y['slot']), y['offset'], types[ y['type'] ])
            ret.update(r)
        return ret
    elif isinstance(value, dict):
        assert len(value) == 1
        k, v = list(value.items())[0]
        v = process_storage_mapping(types, k, v)
        return {key: v}
    elif isinstance(value, list):
        base = types[ value[0] ]
        v = process_storage_dynarray(types, base)
        return {key: v}
    else:
        raise Exception(f'Unsupported value type {value}')

def process_storage(sl):
    """
    Experimental code, not 100% accurate benchmark
    """
    types = {}
    for (tname, tinfo) in (sl['types'] or {}).items():
        tvalue = None
        match tinfo['encoding']:
            case 'inplace':
                if 'members' in tinfo:
                    assert tinfo['label'].startswith('struct')
                    tvalue = tuple(tinfo['members'])
                else:
                    tvalue = tinfo['label']
            case 'mapping':
                tvalue = {tinfo['key']: tinfo['value']}
            case 'bytes':
                tvalue = tinfo['label']
            case 'dynamic_array':
                tvalue = [ tinfo['base'] ]
            case _:
                raise Exception(f'Unsupported type {tinfo}')

        if isinstance(tvalue, str):
            tvalue = tvalue.replace('address payable', 'address')
            tvalue = re.sub(r'contract \w+', 'address', tvalue)
        types[tname] = tvalue

    ret = {}
    for x in sl['storage']:
        r = process_storage_value(types, int(x['slot']), x['offset'], types[ x['type'] ])
        ret.update(r)

    return ret

def process(data, mode):
    if mode == 'storage':
        return process_storage(data['storageLayout'])
    ret = {}
    for x in data['abi']:
        if x['type'] != 'function':
            continue
        args = join_inputs(x['inputs'])
        n = f'{x["name"]}({args})'
        sg = sign(n.encode('ascii'))
        if mode == 'arguments' or mode == 'selectors':
            ret[sg] = args
        elif mode == 'mutability':
            ret[sg] = x.get('stateMutability', '')
        else:
            raise Exception(f'Unknown mode {mode}')

    if mode == 'selectors':
        return list(ret.keys())
    else:
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
        t0 = time.perf_counter()
        r = process(d, mode)
        duration_us = int(time.perf_counter() - t0)
        ret[fname] = [duration_us, r]

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
