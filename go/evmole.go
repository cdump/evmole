package evmole

import (
	"context"
	_ "embed"
	"encoding/binary"
	"fmt"

	"github.com/tetratelabs/wazero"
)

//go:embed evmole.wasm
var evmoleWASM []byte

// FunctionSelectors tries to extract from the given bytecode the function selectors.
func FunctionSelectors(ctx context.Context, code []byte) ([][4]byte, error) {
	// Create a new WebAssembly runtime
	runtime := wazero.NewRuntime(ctx)
	defer func() { _ = runtime.Close(ctx) }()

	instance, err := runtime.Instantiate(ctx, evmoleWASM)
	if err != nil {
		panic(fmt.Errorf("failed to instantiate WASM module: %w", err))
	}
	defer func() { _ = instance.Close(ctx) }()

	gasLimit := 0

	memory := instance.Memory()
	functionSelectorsFunc := instance.ExportedFunction("function_selectors")
	if functionSelectorsFunc == nil {
		panic("could not find exported function: function_selectors")
	}

	codeOffset := uint32(0)
	resultLenOffset := uint32(len(code))
	resultOffset := resultLenOffset + 4
	resultCapacity := uint32(512 * 4)

	// Write input to memory
	ok := memory.Write(codeOffset, code)
	if !ok {
		return nil, fmt.Errorf("failed to write input to memory")
	}

	// Call the WASM function
	results, err := functionSelectorsFunc.Call(ctx,
		uint64(codeOffset), uint64(len(code)), uint64(gasLimit),
		uint64(resultLenOffset), uint64(resultOffset), uint64(resultCapacity))
	if err != nil {
		return nil, fmt.Errorf("failed to call function_selectors: %w", err)
	}

	if status := uint32(results[0]); status != 0 {
		return nil, fmt.Errorf("error: status=%d", status)
	}

	// Read the actual result length
	rawResultLen, ok := memory.Read(resultLenOffset, 4)
	if !ok {
		return nil, fmt.Errorf("failed to read result length")
	}
	resultLen := binary.LittleEndian.Uint32(rawResultLen)

	// Success, read the result
	result, ok := memory.Read(resultOffset, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from memory")
	}

	// Convert to [][4]byte
	selectors := make([][4]byte, len(result)/4)
	for i := 0; i < len(result); i += 4 {
		copy(selectors[i/4][:], result[i:i+4])
	}

	return selectors, nil
}
