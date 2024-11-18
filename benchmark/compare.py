import argparse
import json
import math
import pathlib
import re


def load_data(btype: str, dname: str, providers: list[str], results_dir: str) -> tuple[list, list]:
    data = []
    times = []
    for pname in providers:
        with open(f'{results_dir}/{pname}.{btype}_{dname}.json', 'r') as fh:
            data.append(json.load(fh))
        with open(f'{results_dir}/{pname}.{btype}_{dname}.time', 'r') as fh:
            times.append(float(fh.read()))
    return data, times


def process_selectors(dname: str, providers: list[str], results_dir: str):
    pdata, ptimes = load_data('selectors', dname, providers, results_dir)
    ret = []
    for fname, (_meta, gt) in pdata[0].items():
        gt_set = set(gt)
        data = []
        for i in range(1, len(providers)): # skip ground_truth provider
            d = set(pdata[i][fname][1])
            fp = list(d - gt_set)
            fn = list(gt_set - d)
            data.append([fp, fn])
        ret.append({
            'addr': fname[2:-5], # '0xFF.json' => 'FF'
            'ground_truth': gt,
            'data': data,
        })
    return {'dataset': dname, 'results': ret, 'timings': ptimes[1:]}


def format_time(val: float) -> str:
    return f'{val:.1f}s' if val < 10 else f'{val:.0f}s'


def markdown_selectors(providers: list[str], all_results: list):
    print('<table>')
    print(' <tr>')
    print('  <td>Dataset</td>')
    print('  <td></td>')
    for name in providers[1:]:
        print(f'  <td><a href="benchmark/providers/{name}/"><b><i>{name}</i></b></a></td>')
    print(' </tr>')
    for dataset_idx, dataset_result in enumerate(all_results):
        dataset_name = dataset_result['dataset']
        cnt_contracts = len(dataset_result['results'])
        cnt_funcs = sum(len(x['ground_truth']) for x in dataset_result['results'])
        print(' <tr>')
        print(f'  <td rowspan="5"><b>{dataset_name}</b><br><sub>{cnt_contracts}<br>addresses<br><br>{cnt_funcs}<br>functions</sub></td>')
        print('  <td><i>FP <sub>addrs</sub></i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fp_contracts = sum(len(x['data'][idx][0]) > 0 for x in dataset_result['results'])
            print(f'  <td>{fp_contracts}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>FN <sub>addrs</sub></i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fn_contracts = sum(len(x['data'][idx][1]) > 0 for x in dataset_result['results'])
            print(f'  <td>{fn_contracts}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>FP <sub>funcs</sub></i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fp_signatures = sum(len(x['data'][idx][0]) for x in dataset_result['results'])
            print(f'  <td>{fp_signatures}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>FN <sub>funcs</sub></i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fn_signatures = sum(len(x['data'][idx][1]) for x in dataset_result['results'])
            print(f'  <td>{fn_signatures}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>Time</i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            print(f'  <td>{format_time(dataset_result["timings"][idx])}</td>')
        print(' </tr>')
        if dataset_idx != len(all_results) - 1:
            print(f' <tr><td colspan="{1 + len(providers)}"></td></tr>')
    print('</table>')

def markdown_arguments_or_mutability(providers: list[str], all_results: list, second_results: list|None):
    print('<table>')
    print(' <tr>')
    print('  <td>Dataset</td>')
    print('  <td></td>')
    for name in providers[1:]:
        print(f'  <td><a href="benchmark/providers/{name}/"><b><i>{name}</i></b></a></td>')
    print(' </tr>')
    for dataset_idx, dataset_result in enumerate(all_results):
        dataset_name = dataset_result['dataset']
        cnt_funcs = sum(len(x['func']) for x in dataset_result['results'])
        print(' <tr>')
        print(f'  <td rowspan="{2 if second_results is None else 3}"><b>{dataset_name}</b><br><sub>{cnt_funcs}<br>functions</sub></td>')
        print('  <td><i>Errors</i></td>')
        for provider_idx in range(0, len(providers) - 1): # skip ground_truth provider
            bad_fn = sum(1 - y['data'][provider_idx][0] for x in dataset_result['results'] for y in x['func'])
            print(f'  <td>{(bad_fn*100/cnt_funcs):.1f}%<br><sub>{bad_fn}</sub></td>')
        print(' </tr>')
        if second_results is not None:
            print(' <tr>')
            print('  <td><i>Errors 2nd</i></td>')
            for provider_idx in range(0, len(providers) - 1): # skip ground_truth provider
                bad_fn = sum(1 - y['data'][provider_idx][0] for x in second_results[dataset_idx]['results'] for y in x['func'])
                print(f'  <td>{(bad_fn*100/cnt_funcs):.1f}%<br><sub>{bad_fn}</sub></td>')
            print(' </tr>')
        print(' <tr>')
        print('  <td><i>Time</i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            print(f'  <td>{format_time(dataset_result["timings"][idx])}</td>')
        print(' </tr>')
        if dataset_idx != len(all_results) - 1:
            print(f' <tr><td colspan="{1 + len(providers)}"></td></tr>')
    print('</table>')

def show_selectors(providers: list[str], all_results: list, show_errors: bool):
    for dataset_result in all_results:
        cnt_contracts = len(dataset_result['results'])
        cnt_funcs = sum(len(x['ground_truth']) for x in dataset_result['results'])
        for provider_idx, name in enumerate(providers[1:]):
            fp_signatures = sum(len(x['data'][provider_idx][0]) for x in dataset_result['results'])
            fn_signatures = sum(len(x['data'][provider_idx][1]) for x in dataset_result['results'])
            fp_contracts = sum(len(x['data'][provider_idx][0]) > 0 for x in dataset_result['results'])
            fn_contracts = sum(len(x['data'][provider_idx][1]) > 0 for x in dataset_result['results'])
            print(f'dataset {dataset_result["dataset"]} ({cnt_contracts} contracts, {cnt_funcs} signatures), {name}:')
            print(f'  time: {dataset_result["timings"][provider_idx]:.1f}s')
            print(f'  False Positive: {fp_signatures} signatures, {fp_contracts} contracts')
            print(f'  False Negative: {fn_signatures} signatures, {fn_contracts} contracts')
            if show_errors is not True:
                continue
            print('  errors:')
            for x in dataset_result['results']:
                want = sorted(x['ground_truth'])
                fp = sorted(x['data'][provider_idx][0])
                fn = sorted(x['data'][provider_idx][1])
                if len(fp) > 0 or len(fn) > 0:
                    print('   ', x['addr'])
                    print(f'      want: {want}')
                    print(f'      FP  : {fp}')
                    print(f'      FN  : {fn}')
        print('')

def normalize_args(args: str, rules: set[str]|None) -> str:
    if rules is None:
        return args

    # uint8[3] => uint8,uint8,uint8, also supports uint8[2][3], but not uint8[2][]
    if 'fixed-size-array' in rules:
        def expand(m):
            n = math.prod(int(x) for x in re.findall(r'\[(\d+)\]', m.group(2)))
            return ','.join([m.group(1)] * n) + m.group(3)

        args = re.sub(
            r'([a-z0-9]+)((?:\[\d+\])+)(,|$|\))',
            expand,
            args
        )

    # (bool,address)[],(uint32,uint8) => (bool,address)[],uint32,uint8
    if 'tuples' in rules:
        def f(s):
            s = list(s)
            stack = []
            for i, char in enumerate(s):
                if char == '(':
                    stack.append(i)
                elif char == ')':
                    assert stack, 'Unbalanced parentheses'
                    start = stack.pop()
                    if len(s) == i+1 or s[i+1] != '[':
                        val = ''.join(s[start+1:i])
                        # dynamic types & arrays
                        if 'bytes' not in val and 'string' not in val and '[]' not in val:
                            s[start] = ' '
                            s[i] = ' '

            return ''.join(c for c in s if c != ' ')
        args = f(args)

    # string -> bytes
    if 'string-bytes' in rules:
        args = args.replace('string', 'bytes')
    return args

# for 'arguments' and 'mutability'
def process_functions(tname: str, dname: str, providers: list[str], results_dir: str, normalize_func):
    pdata, ptimes = load_data(tname, dname, providers, results_dir)
    ret = []
    for fname, (_meta, gt) in pdata[0].items():
        func = []
        for sel, gt_val in gt.items():
            if gt_val == '' and tname == 'mutability':
                # old solidity compilers don't output mutability in json abi, skip it
                continue
            data = []
            norm_gt_val = normalize_func(gt_val)
            for i in range(1, len(providers)): # skip ground_truth provider
                val = pdata[i][fname][1][sel]
                norm_val = normalize_func(val)
                if norm_val == norm_gt_val:
                    data.append([1])
                else:
                    data.append([0, val])
            func.append({'s': sel, 'gt': gt_val, 'data': data})

        ret.append({
            'addr': fname[2:-5], # '0xFF.json' => 'FF'
            'func': func,
        })
    return {'dataset': dname, 'results': ret, 'timings': ptimes[1:]}


def mutability_normalize_dummy(x):
    return x

def mutability_normalize(x):
    return 'nonpayable' if (x == 'view' or x == 'pure') else x

def process_mutability(dname: str, providers: list[str], results_dir: str, strict: bool):
    nfn = mutability_normalize_dummy if strict is True else mutability_normalize
    return process_functions('mutability', dname, providers, results_dir, nfn)

def process_arguments(dname: str, providers: list[str], results_dir: str, normalize_rules: set[str]):
    return process_functions('arguments', dname, providers, results_dir, lambda x: normalize_args(x, normalize_rules))

def show_arguments_or_mutability(providers: list[str], all_results: list, show_errors: bool):
    for dataset_result in all_results:
        cnt_contracts = len(dataset_result['results'])
        cnt_funcs = sum(len(x['func']) for x in dataset_result['results'])
        for provider_idx, name in enumerate(providers[1:]):
            good_fn = sum(y['data'][provider_idx][0] for x in dataset_result['results'] for y in x['func'])
            bad_fn = sum(1 - y['data'][provider_idx][0] for x in dataset_result['results'] for y in x['func'])
            print(f'dataset {dataset_result["dataset"]} ({cnt_contracts} contracts, {cnt_funcs} functions), {name}:')
            print(f'  time: {dataset_result["timings"][provider_idx]:.1f}s')
            print(f'  bad:  {bad_fn} functions {(bad_fn*100/cnt_funcs):.2f}%')
            print(f'  good: {good_fn} functions ({(good_fn*100/cnt_funcs):.2f}%)')

            if show_errors is not True:
                continue
            print('  errors:')
            for x in dataset_result['results']:
                for y in x['func']:
                    if len(y['data'][provider_idx]) > 1:
                        assert y['data'][provider_idx][0] == 0
                        want = y['gt']
                        got = y['data'][provider_idx][1]
                        print('   ', x['addr'], y['s'])
                        print(f'      want: {want}')
                        print(f'      got : {got}')
        print('')



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--results-dir', type=str, default=pathlib.Path(__file__).parent / 'results', help='results directory')
    parser.add_argument('--mode', choices=['selectors', 'arguments', 'mutability'], default='selectors', help='mode')
    parser.add_argument('--providers', nargs='+', default=None)
    parser.add_argument('--datasets', nargs='+', default=['largest1k', 'random50k', 'vyper'])
    parser.add_argument('--markdown', nargs='?', default=False, const=True, help='show markdown output')
    parser.add_argument('--show-errors', nargs='?', default=False, const=True, help='show errors')
    parser.add_argument('--normalize-args', nargs='+', required=False, choices=['fixed-size-array', 'tuples', 'string-bytes'], help='normalize arguments rules')
    cfg = parser.parse_args()
    if cfg.providers is None:
        if cfg.mode == 'selectors':
            cfg.providers = ['etherscan', 'evmole-rs', 'evmole-js', 'evmole-py', 'whatsabi', 'sevm', 'evm-hound-rs', 'simple']
        elif cfg.mode == 'arguments':
            cfg.providers = ['etherscan', 'evmole-rs', 'evmole-js', 'evmole-py', 'simple']
        elif cfg.mode == 'mutability':
            cfg.providers = ['etherscan', 'evmole-rs', 'evmole-js', 'evmole-py', 'whatsabi', 'sevm', 'simple']
        else:
            cfg.providers = []
    print('Config:')
    print('\n'.join(f'  {field} = {getattr(cfg, field)}' for field in vars(cfg)), '\n')

    if cfg.mode == 'selectors':
        results = [process_selectors(d, cfg.providers, cfg.results_dir) for d in cfg.datasets]

        if cfg.markdown:
            markdown_selectors(cfg.providers, results)
        else:
            show_selectors(cfg.providers, results, cfg.show_errors)

    if cfg.mode == 'arguments':
        results = [process_arguments(d, cfg.providers, cfg.results_dir, cfg.normalize_args) for d in cfg.datasets]
        if cfg.markdown:
            markdown_arguments_or_mutability(cfg.providers, results, None)
        else:
            show_arguments_or_mutability(cfg.providers, results, cfg.show_errors)

    elif cfg.mode == 'mutability':
        results_strict = [process_mutability(d, cfg.providers, cfg.results_dir, True) for d in cfg.datasets]
        results_not_strict = [process_mutability(d, cfg.providers, cfg.results_dir, False) for d in cfg.datasets]

        if cfg.markdown:
            markdown_arguments_or_mutability(cfg.providers, results_not_strict, results_strict)
        else:
            results = results_not_strict
            for x in results_strict:
                x['dataset'] += '/strict'
                results.append(x)
            show_arguments_or_mutability(cfg.providers, results, cfg.show_errors)
