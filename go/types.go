package evmole

import (
	"encoding/json"
	"fmt"
)

// Contract contains analyzed information about a smart contract.
type Contract struct {
	// Functions is the list of contract functions with their metadata.
	Functions []Function `json:"functions,omitempty"`
	// Storage is the contract storage layout.
	Storage []StorageRecord `json:"storage,omitempty"`
	// Disassembled is the list of disassembled opcodes (offset, instruction).
	Disassembled []Instruction `json:"disassembled,omitempty"`
	// BasicBlocks are sequences of instructions that execute sequentially.
	BasicBlocks []BasicBlock `json:"basic_blocks,omitempty"`
	// ControlFlowGraph represents the program's execution paths.
	ControlFlowGraph *ControlFlowGraph `json:"control_flow_graph,omitempty"`
}

// Instruction represents a disassembled EVM opcode.
type Instruction struct {
	Offset int
	Opcode string
}

// UnmarshalJSON implements custom unmarshaling for Instruction from [offset, opcode] array.
func (i *Instruction) UnmarshalJSON(data []byte) error {
	var arr []json.RawMessage
	if err := json.Unmarshal(data, &arr); err != nil {
		return err
	}
	if len(arr) != 2 {
		return fmt.Errorf("expected array of 2 elements, got %d", len(arr))
	}
	if err := json.Unmarshal(arr[0], &i.Offset); err != nil {
		return err
	}
	return json.Unmarshal(arr[1], &i.Opcode)
}

// MarshalJSON implements custom marshaling for Instruction as [offset, opcode] array.
func (i Instruction) MarshalJSON() ([]byte, error) {
	return json.Marshal([2]any{i.Offset, i.Opcode})
}

// BasicBlock represents a sequence of instructions with single entry and exit.
type BasicBlock struct {
	Start int
	End   int
}

// UnmarshalJSON implements custom unmarshaling for BasicBlock from [start, end] array.
func (b *BasicBlock) UnmarshalJSON(data []byte) error {
	var arr []int
	if err := json.Unmarshal(data, &arr); err != nil {
		return err
	}
	if len(arr) != 2 {
		return fmt.Errorf("expected array of 2 elements, got %d", len(arr))
	}
	b.Start = arr[0]
	b.End = arr[1]
	return nil
}

// MarshalJSON implements custom marshaling for BasicBlock as [start, end] array.
func (b BasicBlock) MarshalJSON() ([]byte, error) {
	return json.Marshal([2]int{b.Start, b.End})
}

// Function represents a public smart contract function.
type Function struct {
	// Selector is the 4-byte function selector as hex string (e.g., "a9059cbb").
	Selector string `json:"selector"`
	// BytecodeOffset is the starting byte offset within EVM bytecode for the function body.
	BytecodeOffset int `json:"bytecode_offset"`
	// Arguments is the function parameter types (e.g., "uint256,address[]").
	Arguments *string `json:"arguments,omitempty"`
	// StateMutability is the function state mutability ("pure", "view", "payable", "nonpayable").
	StateMutability *string `json:"state_mutability,omitempty"`
}

// StorageRecord represents a storage variable record in a contract's storage layout.
type StorageRecord struct {
	// Slot is the storage slot location as hex string (32 bytes).
	Slot string `json:"slot"`
	// Offset is the byte offset within the storage slot (0-31).
	Offset int `json:"offset"`
	// Type is the variable type descriptor.
	Type string `json:"type"`
	// Reads is the list of function selectors that read from this storage location.
	Reads []string `json:"reads"`
	// Writes is the list of function selectors that write to this storage location.
	Writes []string `json:"writes"`
}

// ControlFlowGraph represents the structure and flow of EVM bytecode.
type ControlFlowGraph struct {
	// Blocks is the list of basic blocks in the control flow graph.
	Blocks []Block `json:"blocks"`
}

// Block is a basic block in the control flow graph.
type Block struct {
	// Start is the byte offset where the block's first opcode begins.
	Start int `json:"start"`
	// End is the byte offset where the block's last opcode begins.
	End int `json:"end"`
	// Type indicates how control flow continues after this block.
	Type BlockType `json:"-"`
}

// UnmarshalJSON implements custom unmarshaling for Block.
func (b *Block) UnmarshalJSON(data []byte) error {
	// First unmarshal the basic fields
	type blockAlias struct {
		Start int             `json:"start"`
		End   int             `json:"end"`
		Type  string          `json:"type"`
		Data  json.RawMessage `json:"data,omitempty"`
	}
	var alias blockAlias
	if err := json.Unmarshal(data, &alias); err != nil {
		return err
	}
	b.Start = alias.Start
	b.End = alias.End

	// Parse the block type based on the "type" field
	switch alias.Type {
	case "Terminate":
		var t TerminateData
		if err := json.Unmarshal(alias.Data, &t); err != nil {
			return err
		}
		b.Type = BlockType{Kind: BlockKindTerminate, Terminate: &t}
	case "Jump":
		var j JumpData
		if err := json.Unmarshal(alias.Data, &j); err != nil {
			return err
		}
		b.Type = BlockType{Kind: BlockKindJump, Jump: &j}
	case "Jumpi":
		var j JumpiData
		if err := json.Unmarshal(alias.Data, &j); err != nil {
			return err
		}
		b.Type = BlockType{Kind: BlockKindJumpi, Jumpi: &j}
	case "DynamicJump":
		var dj DynamicJumpData
		if err := json.Unmarshal(alias.Data, &dj); err != nil {
			return err
		}
		b.Type = BlockType{Kind: BlockKindDynamicJump, DynamicJump: &dj}
	case "DynamicJumpi":
		var dj DynamicJumpiData
		if err := json.Unmarshal(alias.Data, &dj); err != nil {
			return err
		}
		b.Type = BlockType{Kind: BlockKindDynamicJumpi, DynamicJumpi: &dj}
	default:
		return fmt.Errorf("unknown block type: %s", alias.Type)
	}
	return nil
}

// MarshalJSON implements custom marshaling for Block with type and data fields.
func (b Block) MarshalJSON() ([]byte, error) {
	type blockJSON struct {
		Start int `json:"start"`
		End   int `json:"end"`
		Type  string `json:"type"`
		Data  any    `json:"data"`
	}

	var data any
	switch b.Type.Kind {
	case BlockKindTerminate:
		data = b.Type.Terminate
	case BlockKindJump:
		data = b.Type.Jump
	case BlockKindJumpi:
		data = b.Type.Jumpi
	case BlockKindDynamicJump:
		data = b.Type.DynamicJump
	case BlockKindDynamicJumpi:
		data = b.Type.DynamicJumpi
	}

	return json.Marshal(blockJSON{
		Start: b.Start,
		End:   b.End,
		Type:  b.Type.Kind.String(),
		Data:  data,
	})
}

// BlockKind represents the kind of block termination.
type BlockKind int

const (
	BlockKindTerminate BlockKind = iota
	BlockKindJump
	BlockKindJumpi
	BlockKindDynamicJump
	BlockKindDynamicJumpi
)

// String returns the JSON type name for the block kind.
func (k BlockKind) String() string {
	switch k {
	case BlockKindTerminate:
		return "Terminate"
	case BlockKindJump:
		return "Jump"
	case BlockKindJumpi:
		return "Jumpi"
	case BlockKindDynamicJump:
		return "DynamicJump"
	case BlockKindDynamicJumpi:
		return "DynamicJumpi"
	default:
		return "Unknown"
	}
}

// BlockType represents the type of control flow at the end of a block.
type BlockType struct {
	Kind         BlockKind
	Terminate    *TerminateData
	Jump         *JumpData
	Jumpi        *JumpiData
	DynamicJump  *DynamicJumpData
	DynamicJumpi *DynamicJumpiData
}

// TerminateData contains data for Terminate block type.
type TerminateData struct {
	// Success indicates whether the termination was successful (true for STOP/RETURN).
	Success bool `json:"success"`
}

// JumpData contains data for Jump block type.
type JumpData struct {
	// To is the destination of the jump.
	To int `json:"to"`
}

// JumpiData contains data for Jumpi block type.
type JumpiData struct {
	// TrueTo is the destination if condition is true.
	TrueTo int `json:"true_to"`
	// FalseTo is the destination if condition is false.
	FalseTo int `json:"false_to"`
}

// DynamicJumpData contains data for DynamicJump block type.
type DynamicJumpData struct {
	// To is the list of possible jump destinations and paths to reach them.
	To []DynamicJump `json:"to"`
}

// DynamicJumpiData contains data for DynamicJumpi block type.
type DynamicJumpiData struct {
	// TrueTo is the list of possible destinations for the true branch.
	TrueTo []DynamicJump `json:"true_to"`
	// FalseTo is the static destination for the false branch.
	FalseTo int `json:"false_to"`
}

// DynamicJump represents a dynamic jump destination with the path taken to reach it.
type DynamicJump struct {
	// Path is the sequence of block offsets representing the path taken to reach this jump.
	Path []int `json:"path"`
	// To is the resolved destination of the jump, if known.
	To *int `json:"to,omitempty"`
}
