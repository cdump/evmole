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
$ python3 compare.py --mode=flow

# Filter by dataset/provider and show errors
python3 compare.py --mode=arguments --datasets largest1k --providers etherscan evmole-py --show-errors

# Normalize argument comparisons
python3 compare.py --mode=arguments --normalize-args fixed-size-array tuples string-bytes

# Output markdown tables
python3 compare.py --mode=selectors --markdown
```

## Control Flow Graph Analysis
The CFG analysis methodology consists of the following steps:

1. Constructing Basic Blocks
   - A basic block is a contiguous subsequence of EVM opcodes with:
     - One entry point (first instruction)
     - Ends at: JUMP, JUMPI, STOP, REVERT, RETURN, INVALID, unknown opcode, or end of code
   - JUMPDEST cannot appear inside a block - it marks the start of a new block

2. Filtering Out Definitely Unreachable Blocks
   A block is definitely unreachable if:
   - It does not begin at pc = 0 (contract start), AND
   - First instruction is not JUMPDEST, AND
   - Previous block does not end with JUMPI whose "false" branch falls through

3. Set Definitions
   - SET_BB: Set of all basic blocks after initial partitioning and removal of invalid blocks
   - SET_CFG: Set of blocks reachable from pc = 0 per CFG algorithm

4. Error Metrics
   - False Positives = (SET_CFG - SET_BB)
     - Blocks CFG claims reachable but not valid basic blocks
     - Should be empty in correct analysis
   - False Negatives = (SET_BB - SET_CFG)
     - Valid blocks not marked reachable by CFG
     - May include legitimate dead code
     - Fewer indicates more precise analysis

## Datasets
See [datasets/README.md](datasets/README.md) for information about how the test datasets were constructed.
