import argparse
import json
import pathlib


def process_dataset(dname: str, providers: list[str], results_dir: str):
    pdata = []
    ptimes = []
    for pname in providers:
        with open(f'{results_dir}/{pname}_{dname}.json', 'r') as fh:
            pdata.append(json.load(fh))
        with open(f'{results_dir}/{pname}_{dname}.time', 'r') as fh:
            ptimes.append(float(fh.read()))

    ret = []
    for fname, gt in pdata[0].items():
        gt_set = set(gt)
        data = []
        for i in range(1, len(providers)): # skip ground_truth provider
            d = set(pdata[i].get(fname, []))
            fp = list(d - gt_set)
            fn = list(gt_set - d)
            data.append([fp, fn])
        ret.append({
            'addr': fname[2:-5], # '0xFF.json' => 'FF'
            'ground_truth': gt,
            'data': data,
        })
    return {'dataset': dname, 'results': ret, 'timings': ptimes[1:]}


def markdown(providers: list[str], all_results: list):
    # :1st_place_medal: :rocket: :zap:
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
        cnt_signatures = sum(len(x['ground_truth']) for x in dataset_result['results'])
        print(' <tr>')
        print(f' <td rowspan="3"><i><b>{dataset_name}</b><br>{cnt_contracts} contracts<br>{cnt_signatures} functions</i></td>')
        print('  <td><i>FP/FN contracts:</i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fp_contracts = sum(len(x['data'][idx][0]) > 0 for x in dataset_result['results'])
            fn_contracts = sum(len(x['data'][idx][1]) > 0 for x in dataset_result['results'])
            print(f'  <td>{fp_contracts} / {fn_contracts}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>FP/FN functions:</i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            fp_signatures = sum(len(x['data'][idx][0]) for x in dataset_result['results'])
            fn_signatures = sum(len(x['data'][idx][1]) for x in dataset_result['results'])
            print(f'  <td>{fp_signatures} / {fn_signatures}</td>')
        print(' </tr>')
        print(' <tr>')
        print('  <td><i>Time:</i></td>')
        for idx in range(0, len(providers) - 1): # skip ground_truth provider
            print(f'  <td>{dataset_result["timings"][idx]}s</td>')
        print(' </tr>')
        if dataset_idx != len(all_results) - 1:
            print(f' <tr><td colspan="{1 + len(providers)}"></td></tr>')
    print('</table>')


def serve_web(listen_host: str, listen_port:int, providers: list[str], all_results: list):
    """
    {
      "providers": ["etherscan", "a", "b"],
      "results": [
        {
            "dataset": "name",
            "timings": [1.2, 0.33], # for every provider
            "results": [
              {
                  "addr": "address",
                  "ground_truth": ["00aabbcc", "ddeeff22"],
                  "data": [
                    [["fp"], ["fn"]], # 1st provider errors
                    [["fp"], ["fn"]], # 2nd provider errors
                  ],
              },
            ]
        }
      ],
    }
    """
    data = {'providers': providers, 'results': all_results}
    json_data = json.dumps(data, separators=(',', ':'))
    async def handle_index(_):
        return web.FileResponse(pathlib.Path(__file__).parent / 'index.html')
    async def handle_res(_):
        return web.Response(body=json_data, headers={'Content-Type': 'application/json'})
    app = web.Application()
    app.add_routes([web.get('/', handle_index), web.get('/res.json', handle_res)])
    web.run_app(app, host=listen_host, port=listen_port)


def show(providers: list[str], all_results: list):
    for dataset_result in all_results:
        cnt_contracts = len(dataset_result['results'])
        cnt_signatures = sum(len(x['ground_truth']) for x in dataset_result['results'])
        for provider_idx, name in enumerate(providers[1:]):
            fp_signatures = sum(len(x['data'][provider_idx][0]) for x in dataset_result['results'])
            fn_signatures = sum(len(x['data'][provider_idx][1]) for x in dataset_result['results'])
            fp_contracts = sum(len(x['data'][provider_idx][0]) > 0 for x in dataset_result['results'])
            fn_contracts = sum(len(x['data'][provider_idx][1]) > 0 for x in dataset_result['results'])
            print(f'dataset {dataset_result["dataset"]} ({cnt_contracts} contracts, {cnt_signatures} signatures), {name}:')
            print(f'  time: {dataset_result["timings"][provider_idx]}s')
            print(f'  False Positive: {fp_signatures} signatures, {fp_contracts} contracts')
            print(f'  False Negative: {fn_signatures} signatures, {fn_contracts} contracts')
            print('')
        print('')
    pass


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--results-dir', type=str, default=pathlib.Path(__file__).parent / 'results', help='results directory')
    parser.add_argument('--providers', nargs='+', default=['etherscan', 'simple', 'whatsabi', 'evm-hound-rs', 'evmole-js', 'evmole-py'])
    parser.add_argument('--datasets', nargs='+', default=['largest1k', 'random50k', 'vyper'])
    parser.add_argument('--web-listen', type=str, default='', help='start webserver to serve results, example: "127.0.0.1:8080"')
    parser.add_argument('--markdown', nargs='?', default=False, const=True, help='show markdown output')
    cfg = parser.parse_args()
    print('Config:')
    print('\n'.join(f'  {field} = {getattr(cfg, field)}' for field in vars(cfg)), '\n')

    if cfg.web_listen != '':
        from aiohttp import web

    results = [process_dataset(d, cfg.providers, cfg.results_dir) for d in cfg.datasets]

    if cfg.markdown:
        markdown(cfg.providers, results)
    else:
        show(cfg.providers, results)

    if cfg.web_listen != '':
        host, port = cfg.web_listen.rsplit(':')
        serve_web(host, int(port), cfg.providers, results)
