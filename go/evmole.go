// Package evmole provides EVM bytecode analysis functionality.
//
// EVMole extracts function selectors, arguments, state mutability,
// storage layout, and control flow graphs from Ethereum Virtual Machine bytecode.
//
// Basic usage:
//
//	// Single contract analysis
//	info, err := evmole.ContractInfo(ctx, bytecode, evmole.Options{Selectors: true})
//
//	// Multiple contracts (efficient - compile WASM once)
//	analyzer, err := evmole.NewAnalyzer(ctx)
//	if err != nil {
//		return err
//	}
//	defer analyzer.Close(ctx)
//
//	for _, bytecode := range contracts {
//		info, err := analyzer.ContractInfo(ctx, bytecode, opts)
//		// ...
//	}
package evmole

import (
	"bytes"
	"compress/gzip"
	"context"
	_ "embed"
	"encoding/json"
	"io"
	"sync"

	"github.com/cdump/evmole/go/internal"
)

//go:embed wasm/evmole.wasm.gz
var wasmBinaryGz []byte

var (
	wasmBinary     []byte
	wasmBinaryOnce sync.Once
	wasmBinaryErr  error
)

func getWasmBinary() ([]byte, error) {
	wasmBinaryOnce.Do(func() {
		r, err := gzip.NewReader(bytes.NewReader(wasmBinaryGz))
		if err != nil {
			wasmBinaryErr = err
			return
		}
		defer r.Close()
		wasmBinary, wasmBinaryErr = io.ReadAll(r)
	})
	return wasmBinary, wasmBinaryErr
}

// Options configures which analyses to perform.
type Options struct {
	// Selectors enables extraction of function selectors.
	Selectors bool
	// Arguments enables extraction of function parameter types.
	Arguments bool
	// StateMutability enables detection of function state mutability.
	StateMutability bool
	// Storage enables extraction of storage layout.
	Storage bool
	// Disassemble enables bytecode disassembly.
	Disassemble bool
	// BasicBlocks enables extraction of basic blocks.
	BasicBlocks bool
	// ControlFlowGraph enables generation of control flow graph.
	ControlFlowGraph bool
}

// Options bitmask constants (must match Rust side)
const (
	optSelectors        uint32 = 1
	optArguments        uint32 = 2
	optStateMutability  uint32 = 4
	optStorage          uint32 = 8
	optDisassemble      uint32 = 16
	optBasicBlocks      uint32 = 32
	optControlFlowGraph uint32 = 64
)

func (o Options) toBitmask() uint32 {
	var mask uint32
	if o.Selectors {
		mask |= optSelectors
	}
	if o.Arguments {
		mask |= optArguments
	}
	if o.StateMutability {
		mask |= optStateMutability
	}
	if o.Storage {
		mask |= optStorage
	}
	if o.Disassemble {
		mask |= optDisassemble
	}
	if o.BasicBlocks {
		mask |= optBasicBlocks
	}
	if o.ControlFlowGraph {
		mask |= optControlFlowGraph
	}
	return mask
}

// Analyzer holds the compiled WASM module for reuse.
// Create once with NewAnalyzer(), use for many contracts.
type Analyzer struct {
	runtime *internal.Runtime
}

// NewAnalyzer creates a new analyzer with AOT-compiled WASM.
// This takes ~50ms for compilation, so create once and reuse.
func NewAnalyzer(ctx context.Context) (*Analyzer, error) {
	wasm, err := getWasmBinary()
	if err != nil {
		return nil, err
	}
	rt, err := internal.NewRuntime(ctx, wasm)
	if err != nil {
		return nil, err
	}
	return &Analyzer{runtime: rt}, nil
}

// Close releases all resources. Call when done with the analyzer.
func (a *Analyzer) Close(ctx context.Context) error {
	return a.runtime.Close(ctx)
}

// ContractInfo analyzes EVM bytecode with the given options.
// Safe to call concurrently from multiple goroutines.
func (a *Analyzer) ContractInfo(ctx context.Context, code []byte, opts Options) (*Contract, error) {
	jsonResult, err := a.runtime.ContractInfo(ctx, code, opts.toBitmask())
	if err != nil {
		return nil, err
	}

	var result Contract
	if err := json.Unmarshal(jsonResult, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// ContractInfo is a convenience function that creates a temporary analyzer.
// For analyzing multiple contracts, use NewAnalyzer() instead.
func ContractInfo(ctx context.Context, code []byte, opts Options) (*Contract, error) {
	a, err := NewAnalyzer(ctx)
	if err != nil {
		return nil, err
	}
	defer a.Close(ctx)
	return a.ContractInfo(ctx, code, opts)
}
