import argparse
import json
import re
import os

import aiohttp
import asyncio

saved_cnt = 0

async def worker(cfg, q: asyncio.Queue):
    global saved_cnt
    async with aiohttp.ClientSession() as session:
        while True:
            addr = await q.get()
            if addr is None:
                break
            out_path = f'{cfg.out_dir}/{addr}.json'

            print(f'Begin {addr} process')
            if os.path.isfile(out_path):
                print(f'Skip {addr}: already exists')
                saved_cnt += 1
                continue

            if cfg.limit != 0 and saved_cnt >= cfg.limit:
                print(f'Skip {addr}: limit reached')
                continue

            req = {'method':'eth_getCode','params':[addr, 'latest'],'id':1,'jsonrpc':'2.0'}
            async with session.post(cfg.rpc_url, json=req) as r:
                code = (await r.json())['result']
            if not cfg.code_regexp.match(code):
                print(f'Skip {addr}: regexp not matched')
                continue

            u = f'https://api.etherscan.io/api?module=contract&action=getabi&apikey={cfg.etherscan_api_key}&address={addr}'
            async with session.get(u) as r:
                abi = (await r.json())['result']
                if abi == 'Contract source code not verified':
                    print(f'Skip {addr}: {abi}')
                    await asyncio.sleep(cfg.threads * 0.3)  # dirty hack: limit ~3 rps for etherscan
                    continue
                assert abi.startswith('['), abi
                abi = json.loads(abi)

            await asyncio.sleep(cfg.threads * 0.3)  # dirty hack: limit ~3 rps for etherscan

            if cfg.limit != 0 and saved_cnt >= cfg.limit:
                print(f'Skip {addr}: limit reached')
                continue

            with open(out_path, 'w') as fh:
                json.dump({'code': code, 'abi': abi}, fh)
            saved_cnt += 1
            print(f'Saved {addr}, {saved_cnt}th address')


async def reader(cfg, q: asyncio.Queue):
    with open(cfg.addrs_list, 'r') as fh:
        for line in fh:
            if cfg.limit != 0 and saved_cnt >= cfg.limit:
                break
            addr = line.rstrip()
            if not addr.startswith('0x'):
                addr = f'0x{addr}'
            await q.put(addr)

    for _ in range(cfg.threads):
        await q.put(None)


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--addrs-list', type=str, required=True, help='file with contract addresses')
    parser.add_argument('--out-dir', type=str, required=True, help='output directory')
    parser.add_argument('--rpc-url', type=str, default='http://127.0.0.1:8545', help='rpc url of ethereum node')
    parser.add_argument('--etherscan-api-key', type=str, required=True, help='etherscan.io api key')
    parser.add_argument('--code-regexp', default='', help='code regexp', type=re.compile)
    parser.add_argument('--limit', type=int, required=False, default=0, help='limit')
    parser.add_argument('--threads', type=int, required=False, default=2, help='threads')
    cfg = parser.parse_args()
    print('Config:')
    print('\n'.join(f'  {field} = {getattr(cfg, field)}' for field in vars(cfg)), '\n')

    q = asyncio.Queue(maxsize = cfg.threads)
    await asyncio.gather(
        reader(cfg, q),
        *[worker(cfg, q) for _ in range(cfg.threads)]
    )
    print(f'Finished, got {saved_cnt} contracts')


if __name__ == '__main__':
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
