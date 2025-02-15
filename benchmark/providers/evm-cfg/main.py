import json
import time
import os
import re
import subprocess
import sys

import pydot

# pydot is VERY slow, 1ms for evm-cfg and 900ms for pydot
def process_slow(output: str) -> list:
    index = output.find("digraph G {")
    assert index != -1
    output = output[index:]

    start_ts = time.monotonic()
    graph = pydot.graph_from_dot_data(output)[0]
    xduration_ms = int((time.monotonic() - start_ts) * 1000)
    print(xduration_ms)

    node2pc = {}
    for node in graph.get_nodes():
        name = node.get_name()
        label = node.get_label()
        if label is not None:
            label = label.strip('"')
            code = [line.split(' ', 1) for line in label.split('\n') if line.startswith('[')]
            assert all(x[1] not in 'JUMPDEST' for x in code[1:])
            node2pc[name] = int(code[0][0][1:-1], 16)

    ret = []
    for edge in graph.get_edges():
        src = edge.get_source()
        dst = edge.get_destination()
        fr = node2pc[src]
        to = node2pc[dst]
        ret.append((fr, to))
    return ret

def process_fast(output: str) -> list:
    ret = []
    addrs = {}
    for line in output.split('\n'):
        node_match = re.match(r'^    (\d+) \[ label = "\[([a-f0-9]+)\]', line)
        if node_match:
            addrs[node_match.group(1)] = int(node_match.group(2), 16)
        else:
            edge_match = re.match(r'^    (\d+) -> (\d+) \[', line)
            if edge_match is not None:
                fr = addrs[edge_match.group(1)]
                to = addrs[edge_match.group(2)]
                ret.append((fr, to))
    return ret


def extract_cfg(code_hex: str):
    start_ts = time.monotonic()
    try:
        output = subprocess.check_output(['evm-cfg', code_hex], timeout=10, text=True)
    except Exception as e:
        print('Err')
        duration_ms = int((time.monotonic() - start_ts) * 1000)
        return (duration_ms, [])
    duration_ms = int((time.monotonic() - start_ts) * 1000)

    # ret = process_slow(output)
    ret = process_fast(output)

    return [duration_ms, sorted(ret)]


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
