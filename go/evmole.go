package evmole

/*
#cgo linux,amd64 LDFLAGS: ${SRCDIR}/staticlibs/linux-amd64/libevmole.a -ldl
#cgo linux,arm64 LDFLAGS: ${SRCDIR}/staticlibs/linux-arm64/libevmole.a -ldl
#cgo darwin,amd64 LDFLAGS: ${SRCDIR}/staticlibs/darwin-amd64/libevmole.a -ldl
#cgo darwin,arm64 LDFLAGS: ${SRCDIR}/staticlibs/darwin-arm64/libevmole.a -ldl
#cgo windows,amd64 LDFLAGS: ${SRCDIR}/staticlibs/windows-amd64/libevmole.a -ldl
#include <stdlib.h>
#include "../evmole.h"
*/
import "C"

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"unsafe"
)

// ContractInfoOptions represents the configuration options for contract analysis
type ContractInfoOptions struct {
	Selectors        bool // Include function selectors
	Arguments        bool // Include function arguments
	StateMutability  bool // Include state mutability
	Storage          bool // Include storage layout
	Disassemble      bool // Include disassembled bytecode
	BasicBlocks      bool // Include basic block analysis
	ControlFlowGraph bool // Include control flow graph
}

// ContractInfo analyzes the provided contract bytecode with custom options
func ContractInfo(bytecode []byte, options ContractInfoOptions) (*Contract, error) {
	cBytecode := C.CString(hex.EncodeToString(bytecode))
	defer C.free(unsafe.Pointer(cBytecode))

	var cOptions C.EvmoleContractInfoOptions

	// Convert Go bools to C ints (0 or 1)
	cOptions.selectors = boolToInt(options.Selectors)
	cOptions.arguments = boolToInt(options.Arguments)
	cOptions.state_mutability = boolToInt(options.StateMutability)
	cOptions.storage = boolToInt(options.Storage)
	cOptions.disassemble = boolToInt(options.Disassemble)
	cOptions.basic_blocks = boolToInt(options.BasicBlocks)
	cOptions.control_flow_graph = boolToInt(options.ControlFlowGraph)

	var cError *C.char
	result := C.evmole_contract_info(cBytecode, cOptions, &cError)

	if result == nil {
		errMsg := C.GoString(cError)
		C.evmole_free(cError)
		return nil, fmt.Errorf("evmole analysis failed: %s", errMsg)
	}

	jsonResult := C.GoString(result)
	C.evmole_free(result)

	// Parse JSON into Contract struct
	var contract Contract
	err := json.Unmarshal([]byte(jsonResult), &contract)
	if err != nil {
		return nil, fmt.Errorf("failed to parse contract info: %w", err)
	}

	return &contract, nil
}

// Analyze is a convenience function that analyzes bytecode with all options enabled
func Analyze(bytecode []byte) (*Contract, error) {
	return ContractInfo(bytecode, ContractInfoOptions{
		Selectors:        true,
		Arguments:        true,
		StateMutability:  true,
		Storage:          true,
		Disassemble:      true,
		BasicBlocks:      true,
		ControlFlowGraph: true,
	})
}

// Helper function to convert a bool to a C int (0 or 1)
func boolToInt(b bool) C.int {
	if b {
		return 1
	}
	return 0
}
