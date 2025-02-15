# Benchmarks

Test accuracy and speed of different EVM bytecode analysis tools

For results, refer to the [main README.md](../README.md#Benchmark).

## Methodology
1. Get N Etherscan-verified contracts, save the bytecode and ABI to `datasets/NAME/ADDR.json`.
2. Extract information from the bytecode using different tools. Each tool runs inside a Docker container and is limited to 1 CPU (see `providers/NAME` and `Makefile`).
3. Assume Etherscan's ABI as ground truth.
4. Compare the results:
   - For selectors: Count [False Positives and False Negatives](https://en.wikipedia.org/wiki/False_positives_and_false_negatives)
   - For arguments/mutability: Count exact matches

## Reproduce
Set the performance mode using `sudo cpupower frequency-set -g performance` and run benchmarks ([GNU Make](https://www.gnu.org/software/make/)) inside the `benchmark/` directory:

```sh
make benchmark-selectors    # Run function selector tests
make benchmark-arguments   # Run argument extraction tests
make benchmark-mutability  # Run state mutability tests
```

To use [Podman](https://podman.io/) instead of Docker:
```sh
DOCKER=podman make benchmark-selectors
```

You can run specific steps; for example:
```sh
# Only build docker-images
$ make build

# Only run tests for selectors (assume docker-images are built)
$ make run-selectors

# Build specific provider
$ make etherscan.build

# Run specific provider/mode/dataset
$ make etherscan.selectors/largest1k
$ make etherscan.arguments/largest1k
```

## Process Results
Use `compare.py` to analyze results:

```sh
# Default mode (selectors)
$ python3 compare.py

# Compare specific mode
$ python3 compare.py --mode=arguments
$ python3 compare.py --mode=mutability

# Filter by dataset/provider and show errors
python3 compare.py --mode=arguments --datasets largest1k --providers etherscan evmole-py --show-errors

# Normalize argument comparisons
python3 compare.py --mode=arguments --normalize-args fixed-size-array tuples string-bytes

# Output markdown tables
python3 compare.py --mode=selectors --markdown
```

## Datasets
See [datasets/README.md](datasets/README.md) for information about how the test datasets were constructed.
