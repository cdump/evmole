import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

GIGAHORSE = '/gigahorse/gigahorse.py'
STORAGE_CLIENT = '/gigahorse/clients/storage_layout.dl'
# gigahorse's fallback and receive functions (logic/global_only.dl); real selectors are unpadded, so a real 0 is "0x0"
PSEUDO_SIGHASHES = {'0x00000000', '0xeeeeeeee'}


def read_rows(out_dir: str, relation: str) -> list[list[str]]:
    try:
        with open(f'{out_dir}/{relation}.csv') as fh:
            return [line.rstrip('\n').split('\t') for line in fh]
    except FileNotFoundError:
        return []


def selectors(out_dir: str) -> list[str]:
    sigs = {int(sig, 16) for _, sig in read_rows(out_dir, 'PublicFunction') if sig not in PSEUDO_SIGHASHES}
    return [f'{s:08x}' for s in sorted(sigs)]


def storage(out_dir: str) -> dict[str, str]:
    structs = {sid: f'struct_{n}_fields' for sid, n in read_rows(out_dir, 'StorageLayoutStruct')}

    def fmt(t: str) -> str:
        t = t.replace('mapping (', 'mapping(')
        return re.sub(r'struct_\d+', lambda m: structs.get(m[0], m[0]), t)

    ret = {f'{int(slot, 16):064x}_{off}': fmt(t) for slot, off, t in read_rows(out_dir, 'StorageLayoutVariable')}
    # a data structure at the same slot wins over the plain word holding its length
    ret.update({f'{int(slot, 16):064x}_0': fmt(t) for slot, t in read_rows(out_dir, 'StorageOffset_Type')})
    return ret


def flow(out_dir: str) -> list[list[int]]:
    # TMP: fake edges from the entry to every block the global analysis reaches (mapped back to original blocks).
    # compare.py only scores the blocks reachable from 0, but real edges are required: output global.BlockEdge
    # mapped through revertCloner.Block_OriginalBlock from a patched logic/main.dl
    blocks = {int(row[0], 16) for row in read_rows(out_dir, 'Analytics_ReachableBlocks')}
    return [[0, b] for b in sorted(blocks - {0})]


def analyze(code: str, mode: str, tmp: str):
    with open(f'{tmp}/c.hex', 'w') as fh:
        fh.write(code)
    work_dir = f'{tmp}/w'
    cmd = [sys.executable, GIGAHORSE, '-q', '-j', '1', '--reuse_datalog_bin', '--skip_sig_resolution',
           '-w', work_dir, '-r', f'{work_dir}/results.json']
    # inlining only helps client analyses; selectors and reachable blocks come straight from the decompiler
    cmd += ['-C', STORAGE_CLIENT] if mode == 'storage' else ['--disable_inline']

    start_ts = time.perf_counter_ns()
    subprocess.run([*cmd, f'{tmp}/c.hex'], stdout=subprocess.DEVNULL)
    duration_us = int((time.perf_counter_ns() - start_ts) / 1000)

    try:
        with open(f'{work_dir}/results.json') as fh:
            flags = json.load(fh)[0][2]  # TIMEOUT, ERROR, CLIENT TIMEOUT or CLIENT ERROR
    except (FileNotFoundError, IndexError, ValueError):
        flags = ['ERROR']

    if flags:  # partial output of a failed or timed out run gets no credit
        ret = {} if mode == 'storage' else []
    else:
        ret = {'selectors': selectors, 'storage': storage, 'flow': flow}[mode](f'{work_dir}/c/out')
    # gigahorse skips contracts whose work dir exists, and each run leaves megabytes of IR facts
    shutil.rmtree(work_dir, ignore_errors=True)
    return [duration_us, ret]


if len(sys.argv) < 4:
    print('Usage: python3 main.py MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]')
    sys.exit(1)

mode = sys.argv[1]
indir = sys.argv[2]
outfile = sys.argv[3]

assert mode in ('selectors', 'storage', 'flow'), f'unsupported mode {mode}'

ret = {}
with tempfile.TemporaryDirectory() as tmp:
    for fname in os.listdir(indir):
        with open(f'{indir}/{fname}', 'r') as fh:
            d = json.load(fh)
        ret[fname] = analyze(d.get('code', d.get('runtimeBytecode')), mode, tmp)

with open(outfile, 'w') as fh:
    json.dump(ret, fh)
