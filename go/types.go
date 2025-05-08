package evmole

import (
	"encoding/json"
	"fmt"
)

// Function represents a public smart contract function
type Function struct {
	Selector        string `json:"selector"`        // Function selector as a 4-byte hex string
	BytecodeOffset  int    `json:"bytecodeOffset"`  // The starting byte offset within the EVM bytecode for the function body
	Arguments       string `json:"arguments"`       // Function arguments in canonical format (e.g., "uint256,address[]")
	StateMutability string `json:"stateMutability"` // Function's state mutability (e.g., "pure", "view", "payable", "nonpayable")
}

// StorageRecord represents a storage variable record in a contract's storage layout
type StorageRecord struct {
	Slot   string   `json:"slot"`   // Storage slot as a hex string
	Offset int      `json:"offset"` // Byte offset within the storage slot (0-31)
	Type   string   `json:"type"`   // Variable type (e.g., "uint256", "mapping(address => uint256)")
	Reads  []string `json:"reads"`  // Function selectors that read from this storage location
	Writes []string `json:"writes"` // Function selectors that write to this storage location
}

// Block represents a basic block in the control flow graph representing a sequence of instructions
// with a single entry point and a single exit point
type Block struct {
	// Start is the byte offset where the block's first opcode begins
	Start int `json:"start"`
	// End is the byte offset where the block's last opcode begins
	End int `json:"end"`
	// Type indicates how control flow continues after this block
	Type string `json:"type"`
	// Data contains the type-specific block data
	Data BlockData `json:"data"`
}

// BlockData represents the different types of control flow that can occur at the end of a block
type BlockData interface {
	isBlockData()
}

// TerminateData represents a block that ends with terminating instruction
type TerminateData struct {
	// Success indicates whether the termination was successful (true for STOP/RETURN)
	Success bool `json:"success"`
}

// JumpData represents a block that ends with an unconditional jump
type JumpData struct {
	// To is the destination of the jump
	To int `json:"to"`
}

// JumpiData represents a block that ends with a conditional jump
type JumpiData struct {
	// TrueTo is the destination if condition is true
	TrueTo int `json:"true_to"`
	// FalseTo is the destination if condition is false
	FalseTo int `json:"false_to"`
}

// DynamicJump represents a dynamic jump destination with the path taken to reach it
type DynamicJump struct {
	// Path is the sequence of block offsets representing the path taken to reach this jump
	Path []int `json:"path"`
	// To is the resolved destination of the jump, if known
	To *int `json:"to,omitempty"`
}

// DynamicJumpData represents a block that ends with an unconditional jump to a dynamically computed destination
type DynamicJumpData struct {
	// To contains possible jump destinations and paths to reach them
	To []DynamicJump `json:"to"`
}

// DynamicJumpiData represents a block that ends with a conditional jump where the true branch is dynamic
type DynamicJumpiData struct {
	// TrueTo contains possible jump destinations and paths for the true branch
	TrueTo []DynamicJump `json:"true_to"`
	// FalseTo is the static destination for the false branch
	FalseTo int `json:"false_to"`
}

// Implement isBlockData for all concrete types
func (*TerminateData) isBlockData()    {}
func (*JumpData) isBlockData()         {}
func (*JumpiData) isBlockData()        {}
func (*DynamicJumpData) isBlockData()  {}
func (*DynamicJumpiData) isBlockData() {}

// MarshalJSON implements custom JSON marshaling for Block
func (b Block) MarshalJSON() ([]byte, error) {
	// Create a temporary struct that matches our desired JSON structure
	var temp struct {
		Start int    `json:"start"`
		End   int    `json:"end"`
		Type  string `json:"type"`
		Data  any    `json:"data"`
	}

	// Determine the type and data based on the concrete type of b.Data
	switch d := b.Data.(type) {
	case *TerminateData:
		temp.Type = "Terminate"
		temp.Data = d
	case *JumpData:
		temp.Type = "Jump"
		temp.Data = d
	case *JumpiData:
		temp.Type = "Jumpi"
		temp.Data = d
	case *DynamicJumpData:
		temp.Type = "DynamicJump"
		temp.Data = d
	case *DynamicJumpiData:
		temp.Type = "DynamicJumpi"
		temp.Data = d
	default:
		return nil, fmt.Errorf("unknown block data type: %T", b.Data)
	}

	// Create and marshal the alias struct
	return json.Marshal(temp)
}

// UnmarshalJSON implements custom JSON unmarshaling for Block
func (b *Block) UnmarshalJSON(data []byte) error {
	// First unmarshal into a temporary structure
	var temp struct {
		Start int             `json:"start"`
		End   int             `json:"end"`
		Type  string          `json:"type"`
		Data  json.RawMessage `json:"data"`
	}

	if err := json.Unmarshal(data, &temp); err != nil {
		return err
	}

	b.Start = temp.Start
	b.End = temp.End
	b.Type = temp.Type

	// Based on the Type field, unmarshal Data into the appropriate concrete type
	switch temp.Type {
	case "Terminate":
		var d TerminateData
		if err := json.Unmarshal(temp.Data, &d); err != nil {
			return err
		}
		b.Data = &d
	case "Jump":
		var d JumpData
		if err := json.Unmarshal(temp.Data, &d); err != nil {
			return err
		}
		b.Data = &d
	case "Jumpi":
		var d JumpiData
		if err := json.Unmarshal(temp.Data, &d); err != nil {
			return err
		}
		b.Data = &d
	case "DynamicJump":
		var d DynamicJumpData
		if err := json.Unmarshal(temp.Data, &d); err != nil {
			return err
		}
		b.Data = &d
	case "DynamicJumpi":
		var d DynamicJumpiData
		if err := json.Unmarshal(temp.Data, &d); err != nil {
			return err
		}
		b.Data = &d
	default:
		return fmt.Errorf("unknown block type: %s", temp.Type)
	}

	return nil
}

// ControlFlowGraph represents the control flow graph of the contract bytecode
type ControlFlowGraph struct {
	Blocks []Block `json:"blocks"` // List of basic blocks in the control flow graph
}

// DisassembledInstruction represents a single disassembled instruction
type DisassembledInstruction struct {
	Offset      int
	Instruction string
}

// MarshalJSON implements custom JSON marshaling to output [offset, instruction]
func (d DisassembledInstruction) MarshalJSON() ([]byte, error) {
	return json.Marshal([2]any{d.Offset, d.Instruction})
}

// UnmarshalJSON implements custom JSON unmarshaling from [offset, instruction]
func (d *DisassembledInstruction) UnmarshalJSON(data []byte) error {
	var raw [2]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	if err := json.Unmarshal(raw[0], &d.Offset); err != nil {
		return fmt.Errorf("unmarshal offset: %w", err)
	}
	if err := json.Unmarshal(raw[1], &d.Instruction); err != nil {
		return fmt.Errorf("unmarshal instruction: %w", err)
	}

	return nil
}

// BasicBlock represents a block's start and end offsets
type BasicBlock struct {
	Start int
	End   int
}

// MarshalJSON implements custom JSON marshaling to output [start, end]
func (b BasicBlock) MarshalJSON() ([]byte, error) {
	return json.Marshal([2]int{b.Start, b.End})
}

// UnmarshalJSON implements custom JSON unmarshaling from [start, end]
func (b *BasicBlock) UnmarshalJSON(data []byte) error {
	var raw [2]int
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}
	b.Start = raw[0]
	b.End = raw[1]
	return nil
}

// Contract represents a smart contract and the results of its analysis
type Contract struct {
	Functions        []Function                `json:"functions,omitempty"`        // List of contract functions
	Storage          []StorageRecord           `json:"storage,omitempty"`          // Contract storage layout
	Disassembled     []DisassembledInstruction `json:"disassembled,omitempty"`     // Disassembled code
	BasicBlocks      []BasicBlock              `json:"basicBlocks,omitempty"`      // Basic blocks representing sequences of instructions that execute sequentially
	ControlFlowGraph *ControlFlowGraph         `json:"controlFlowGraph,omitempty"` // Control flow graph
}
