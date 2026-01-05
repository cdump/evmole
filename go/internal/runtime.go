package internal

import (
	"context"
	"encoding/binary"
	"errors"
	"sync"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// Runtime manages the WASM module execution.
type Runtime struct {
	runtime wazero.Runtime
	module  api.Module
	alloc   api.Function
	dealloc api.Function
	analyze api.Function

	// WASM is single-threaded, need mutex for concurrent access
	mu sync.Mutex
}

// NewRuntime creates a new WASM runtime with AOT compilation.
// This takes ~50ms for compilation, so create once and reuse.
func NewRuntime(ctx context.Context, wasmBinary []byte) (*Runtime, error) {
	// Use AOT compilation for better performance
	r := wazero.NewRuntimeWithConfig(ctx, wazero.NewRuntimeConfigCompiler())

	module, err := r.Instantiate(ctx, wasmBinary)
	if err != nil {
		r.Close(ctx)
		return nil, err
	}

	alloc := module.ExportedFunction("wasm_alloc")
	if alloc == nil {
		r.Close(ctx)
		return nil, errors.New("wasm_alloc function not found in WASM module")
	}

	dealloc := module.ExportedFunction("wasm_dealloc")
	if dealloc == nil {
		r.Close(ctx)
		return nil, errors.New("wasm_dealloc function not found in WASM module")
	}

	analyze := module.ExportedFunction("contract_info")
	if analyze == nil {
		r.Close(ctx)
		return nil, errors.New("contract_info function not found in WASM module")
	}

	return &Runtime{
		runtime: r,
		module:  module,
		alloc:   alloc,
		dealloc: dealloc,
		analyze: analyze,
	}, nil
}

// Close releases all runtime resources.
func (r *Runtime) Close(ctx context.Context) error {
	return r.runtime.Close(ctx)
}

// ContractInfo analyzes bytecode and returns the JSON result.
func (r *Runtime) ContractInfo(ctx context.Context, code []byte, opts uint32) ([]byte, error) {
	if len(code) == 0 {
		return nil, errors.New("empty bytecode")
	}

	r.mu.Lock()
	defer r.mu.Unlock()

	// 1. Allocate memory for input code
	codePtrResult, err := r.alloc.Call(ctx, uint64(len(code)))
	if err != nil {
		return nil, err
	}
	if len(codePtrResult) == 0 || codePtrResult[0] == 0 {
		return nil, errors.New("failed to allocate memory for bytecode")
	}
	codePtr := uint32(codePtrResult[0])

	// Ensure code memory is freed
	defer r.dealloc.Call(ctx, uint64(codePtr), uint64(len(code)))

	// 2. Copy code to WASM memory
	if !r.module.Memory().Write(codePtr, code) {
		return nil, errors.New("failed to write bytecode to WASM memory")
	}

	// 3. Call analyze function
	resultPtrArr, err := r.analyze.Call(ctx, uint64(codePtr), uint64(len(code)), uint64(opts))
	if err != nil {
		return nil, err
	}
	if len(resultPtrArr) == 0 || resultPtrArr[0] == 0 {
		return nil, errors.New("contract_info returned null pointer")
	}
	resultPtr := uint32(resultPtrArr[0])

	// 4. Read result length (first 4 bytes) then data
	lenBytes, ok := r.module.Memory().Read(resultPtr, 4)
	if !ok {
		return nil, errors.New("failed to read result length from WASM memory")
	}
	resultLen := binary.LittleEndian.Uint32(lenBytes)

	result, ok := r.module.Memory().Read(resultPtr+4, resultLen)
	if !ok {
		return nil, errors.New("failed to read result from WASM memory")
	}

	// Make a copy since the memory might be reused
	resultCopy := make([]byte, len(result))
	copy(resultCopy, result)

	// 5. Deallocate result (length prefix + data)
	r.dealloc.Call(ctx, uint64(resultPtr), uint64(resultLen)+4)

	return resultCopy, nil
}
