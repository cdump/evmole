# EVMole Go

Go bindings for [EVMole](https://github.com/cdump/evmole) - a library that extracts information from Ethereum Virtual Machine (EVM) bytecode, including function selectors, arguments, state mutability, storage layout, and control flow graphs.

## Features

- **No CGO required** - Pure Go implementation using WebAssembly (wazero)
- **Cross-platform** - Works on Linux, macOS, Windows (amd64, arm64)
- **Reusable analyzer** - Create once, analyze many contracts efficiently

## Installation

```bash
go get github.com/cdump/evmole/go
```

## Usage

### Basic Usage

```go
package main

import (
    "context"
    "encoding/hex"
    "fmt"
    "log"

    "github.com/cdump/evmole/go"
)

func main() {
    ctx := context.Background()

    bytecode, _ := hex.DecodeString("6080604052...") // Your contract bytecode

    info, err := evmole.ContractInfo(ctx, bytecode, evmole.Options{
        Selectors:       true,
        Arguments:       true,
        StateMutability: true,
    })
    if err != nil {
        log.Fatal(err)
    }

    for _, fn := range info.Functions {
        fmt.Printf("Selector: %s\n", fn.Selector)
        if fn.Arguments != nil {
            fmt.Printf("  Arguments: %s\n", *fn.Arguments)
        }
        if fn.StateMutability != nil {
            fmt.Printf("  State Mutability: %s\n", *fn.StateMutability)
        }
    }
}
```

### Efficient Batch Processing

For analyzing multiple contracts, create an analyzer once and reuse it:

```go
// Create analyzer
analyzer, err := evmole.NewAnalyzer(ctx)
if err != nil {
    log.Fatal(err)
}
defer analyzer.Close(ctx)

// Analyze multiple contracts efficiently
for _, bytecode := range contracts {
    info, err := analyzer.ContractInfo(ctx, bytecode, evmole.Options{
        Selectors: true,
    })
    if err != nil {
        log.Printf("Error: %v", err)
        continue
    }
    // Process info...
}
```

### Storage Layout Analysis

```go
info, err := evmole.ContractInfo(ctx, bytecode, evmole.Options{
    Storage: true,  // Note: also enables Selectors and Arguments
})

for _, record := range info.Storage {
    fmt.Printf("Slot: %s\n", record.Slot)
    fmt.Printf("  Type: %s\n", record.Type)
    fmt.Printf("  Offset: %d\n", record.Offset)
    fmt.Printf("  Reads: %v\n", record.Reads)
    fmt.Printf("  Writes: %v\n", record.Writes)
}
```

### Control Flow Graph

```go
info, err := evmole.ContractInfo(ctx, bytecode, evmole.Options{
    ControlFlowGraph: true,  // Note: also enables BasicBlocks
})

for _, block := range info.ControlFlowGraph.Blocks {
    fmt.Printf("Block %d-%d: ", block.Start, block.End)
    switch block.Type.Kind {
    case evmole.BlockKindTerminate:
        fmt.Printf("Terminate(success=%v)\n", block.Type.Terminate.Success)
    case evmole.BlockKindJump:
        fmt.Printf("Jump(to=%d)\n", block.Type.Jump.To)
    case evmole.BlockKindJumpi:
        fmt.Printf("Jumpi(true=%d, false=%d)\n",
            block.Type.Jumpi.TrueTo, block.Type.Jumpi.FalseTo)
    }
}
```

### Disassembly

```go
info, err := evmole.ContractInfo(ctx, bytecode, evmole.Options{
    Disassemble: true,
})

for _, instr := range info.Disassembled {
    fmt.Printf("%04x: %s\n", instr.Offset, instr.Opcode)
}
```

## API Reference

### Options

| Field | Description |
|-------|-------------|
| `Selectors` | Extract function selectors (4-byte signatures) |
| `Arguments` | Extract function parameter types |
| `StateMutability` | Detect function state mutability (pure/view/payable/nonpayable) |
| `Storage` | Extract storage layout (enables Selectors and Arguments) |
| `Disassemble` | Disassemble bytecode into opcodes |
| `BasicBlocks` | Extract basic blocks |
| `ControlFlowGraph` | Generate control flow graph (enables BasicBlocks) |

### Types

#### Contract
```go
type Contract struct {
    Functions        []Function
    Storage          []StorageRecord
    Disassembled     []Instruction
    BasicBlocks      []BasicBlock
    ControlFlowGraph *ControlFlowGraph
}
```

#### Function
```go
type Function struct {
    Selector        string   // e.g., "a9059cbb"
    BytecodeOffset  int      // Entry point in bytecode
    Arguments       *string  // e.g., "uint256,address[]"
    StateMutability *string  // "pure", "view", "payable", "nonpayable"
}
```

#### StorageRecord
```go
type StorageRecord struct {
    Slot   string   // 32-byte hex slot
    Offset int      // Byte offset within slot (0-31)
    Type   string   // e.g., "uint256", "mapping(address => uint256)"
    Reads  []string // Function selectors that read
    Writes []string // Function selectors that write
}
```

## Thread Safety

The `Analyzer` type is safe for concurrent use from multiple goroutines. Internally, it uses a mutex to serialize WASM execution (since WASM is single-threaded). For high-concurrency workloads, consider creating a pool of analyzers.
