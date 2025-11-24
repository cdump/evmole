package evmole

import (
	"context"
	_ "embed"
	"encoding/hex"
	"encoding/json"
	"fmt"

	"github.com/tetratelabs/wazero"
)

//go:embed evmole.wasm
var evmoleWASM []byte

// Selector is a 4-byte function selector
type Selector [4]byte

// String returns the selector as a hex string
func (s Selector) String() string {
	return hex.EncodeToString(s[:])
}

// Bytes returns the selector as a byte slice
func (s Selector) Bytes() []byte {
	return s[:]
}

// UnmarshalJSON unmarshals a JSON string into a Selector
func (s *Selector) UnmarshalText(data []byte) error {
	_, err := hex.Decode(s[:], data)
	return err
}

// MarshalText marshals the selector as a hex string
func (s Selector) MarshalText() ([]byte, error) {
	return []byte(s.String()), nil
}

// ContractInfoOptions configures what information to extract from contract bytecode
type ContractInfoOptions struct {
	// Selectors enables extraction of function selectors
	Selectors bool
	// Arguments enables extraction of function arguments
	Arguments bool
	// StateMutability enables extraction of function state mutability
	StateMutability bool
	// Storage enables extraction of contract storage layout
	Storage bool
}

// Function represents a public smart contract function
type Function struct {
	// Selector is the 4-byte function selector as hex string (e.g., "aabbccdd")
	Selector Selector `json:"selector"`
	// BytecodeOffset is the starting byte offset within the EVM bytecode for the function body
	BytecodeOffset int `json:"bytecodeOffset"`
	// Arguments are the function argument types in canonical format (e.g., "uint256,address[]")
	// nil if arguments were not extracted
	Arguments *string `json:"arguments,omitempty"`
	// StateMutability is the function's state mutability ("pure", "view", "payable", or "nonpayable")
	// nil if state mutability was not extracted
	StateMutability *string `json:"stateMutability,omitempty"`
}

// StorageRecord represents a storage variable record in a smart contract's storage layout
type StorageRecord struct {
	// Slot is the storage slot number as hex string (e.g., "0", "1b")
	Slot string `json:"slot"`
	// Offset is the byte offset within the storage slot (0-31)
	Offset uint8 `json:"offset"`
	// Type is the variable type (e.g., "uint256", "mapping(address => uint256)", "bytes32")
	Type string `json:"type"`
	// Reads is a list of function selectors that read from this storage location
	Reads []Selector `json:"reads"`
	// Writes is a list of function selectors that write to this storage location
	Writes []Selector `json:"writes"`
}

// Contract contains analyzed information about a smart contract
type Contract struct {
	// Functions is the list of detected contract functions
	// nil if functions were not extracted
	Functions []Function `json:"functions,omitempty"`
	// Storage is the list of contract storage records
	// nil if storage layout was not extracted
	Storage []StorageRecord `json:"storage,omitempty"`
}

// ContractInfo extracts information about a smart contract from its EVM bytecode.
//
// Parameters:
//   - ctx: Context for cancellation and timeout
//   - code: Runtime bytecode as raw bytes
//   - options: Optional configuration specifying what data to extract. If nil, extracts selectors only.
//
// Returns:
//   - Contract object containing the requested smart contract information
//   - Error if extraction fails
//
// Example:
//
//	code, _ := hex.DecodeString("6080604052...")
//	contract, err := ContractInfo(ctx, code, &ContractInfoOptions{
//	    Selectors: true,
//	    StateMutability: true,
//	})
func ContractInfo(ctx context.Context, code []byte, options *ContractInfoOptions) (*Contract, error) {
	if options == nil {
		options = &ContractInfoOptions{Selectors: true}
	}

	// Create a new WebAssembly runtime
	runtime := wazero.NewRuntime(ctx)
	defer func() { _ = runtime.Close(ctx) }()

	instance, err := runtime.Instantiate(ctx, evmoleWASM)
	if err != nil {
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}
	defer func() { _ = instance.Close(ctx) }()

	memory := instance.Memory()
	contractInfoFunc := instance.ExportedFunction("contract_info")
	if contractInfoFunc == nil {
		return nil, fmt.Errorf("could not find exported function: contract_info")
	}

	// Prepare options byte (bitflags)
	var optionsByte uint8
	if options.Selectors {
		optionsByte |= 1 << 0
	}
	if options.Arguments {
		optionsByte |= 1 << 1
	}
	if options.StateMutability {
		optionsByte |= 1 << 2
	}
	if options.Storage {
		optionsByte |= 1 << 3
	}

	// Memory layout:
	// [0..len(code)]: input code
	// [len(code)]: options byte
	// [len(code)+1..len(code)+5]: result length (u32)
	// [len(code)+5..]: result JSON
	codeOffset := uint32(0)
	optionsOffset := uint32(len(code))
	resultLenOffset := optionsOffset + 1
	resultOffset := resultLenOffset + 4
	resultCapacity := uint32(1024 * 1024) // 1MB buffer for result

	// Write input to memory
	if !memory.Write(codeOffset, code) {
		return nil, fmt.Errorf("failed to write code to memory")
	}
	if !memory.Write(optionsOffset, []byte{optionsByte}) {
		return nil, fmt.Errorf("failed to write options to memory")
	}

	// Call the WASM function
	results, err := contractInfoFunc.Call(ctx,
		uint64(codeOffset), uint64(len(code)),
		uint64(optionsOffset),
		uint64(resultLenOffset), uint64(resultOffset), uint64(resultCapacity))
	if err != nil {
		return nil, fmt.Errorf("failed to call contract_info: %w", err)
	}

	if status := uint32(results[0]); status != 0 {
		return nil, fmt.Errorf("contract_info error: status=%d", status)
	}

	// Read the result length
	rawResultLen, ok := memory.Read(resultLenOffset, 4)
	if !ok {
		return nil, fmt.Errorf("failed to read result length")
	}
	resultLen := uint32(rawResultLen[0]) | uint32(rawResultLen[1])<<8 |
		uint32(rawResultLen[2])<<16 | uint32(rawResultLen[3])<<24

	// Read the result JSON
	resultJSON, ok := memory.Read(resultOffset, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from memory")
	}

	// Parse JSON result
	var contract Contract
	if err := json.Unmarshal(resultJSON, &contract); err != nil {
		return nil, fmt.Errorf("failed to parse result JSON: %w", err)
	}

	return &contract, nil
}

func FunctionSelectors(ctx context.Context, code []byte) ([]Selector, error) {
	contract, err := ContractInfo(ctx, code, &ContractInfoOptions{Selectors: true})
	if err != nil {
		return nil, err
	}

	selectors := make([]Selector, len(contract.Functions))
	for i, fn := range contract.Functions {
		selectors[i] = fn.Selector
	}
	return selectors, nil
}
