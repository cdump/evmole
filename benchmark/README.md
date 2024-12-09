# Benchmarks

Test accuracy and speed of different function-signature and arguments extractors

For results, refer to the [main README.md](../README.md#Benchmark).

## Methodology
1. Get N Etherscan-verified contracts, save the bytecode and ABI to `datasets/NAME/ADDR.json`.
2. Extract function signatures/arguments/state mutability from the bytecode. Each tool runs inside a Docker container and is limited to 1 CPU (see `providers/NAME` and `Makefile`).
3. Assume Etherscan's ABI as ground truth.
4. Compare the results with it and count [False Positives and False Negatives](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) for signatures and count correct results (strings equal) for arguments and state mutability.

## Reproduce
Set the performance mode using `sudo cpupower frequency-set -g performance` and run `make benchmark-selectors` or `make benchmark-arguments` ([GNU Make](https://www.gnu.org/software/make/)) inside the `benchmark/` directory.

To use [Podman](https://podman.io/) instead of Docker: `DOCKER=podman make benchmark-selectors`


You can run only specific step; for example:
```sh
# Only build docker-images
$ make build

# Only run tests for selectors (assume that docker-images are already built)
$ make run-selectors

# Build `etherscan` docker image
$ make etherscan.build

# Run `etherscan` on dataset `largest1k` to extract function selectors
$ make etherscan.selectors/largest1k

# Run `etherscan` on dataset `largest1k` to extract function arguments
$ make etherscan.arguments/largest1k
```

To process results run `compare.py`:
```sh
# default mode: compare 'selectors' results
$ python3 compare.py

# compare 'arguments' results
$ python3 compare.py --mode=arguments

# compare 'arguments' results for specified providers and datasets, show errors
$ python3 compare.py --mode=arguments --datasets largest1k --providers etherscan evmole-py --show-errors

# compare in web-browser
$ ../.venv/bin/python3 compare.py --web-listen 127.0.0.1:8080 
```


## How datasets was constructed
See [datasets/README.md](datasets/ README)
