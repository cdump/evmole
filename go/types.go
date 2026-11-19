package evmole

import (
	"encoding/json"
	"fmt"
)

// Contract contains analyzed information about a smart contract.
type Contract struct {
	// Functions is the list of contract functions with their metadata.
	Functions []Function `json:"functions,omitempty"`
	// Storage is the persistent contract storage layout.
	Storage []StorageRecord `json:"storage,omitempty"`
	// TransientStorage contains transient storage accesses and inferred layout.
	TransientStorage []StorageRecord `json:"transient_storage,omitempty"`
	// Disassembled is the list of disassembled opcodes (offset, instruction).
	Disassembled []Instruction `json:"disassembled,omitempty"`
	// BasicBlocks are sequences of instructions that execute sequentially.
	BasicBlocks []BasicBlock `json:"basic_blocks,omitempty"`
	// ControlFlowGraph represents the program's execution paths.
	ControlFlowGraph *ControlFlowGraph `json:"control_flow_graph,omitempty"`
	// Metadata is terminal CBOR metadata, when requested and valid.
	Metadata *CborMetadata `json:"metadata,omitempty"`
	// Proxy is nil unless requested. An empty slice means no implementation source was found.
	Proxy []ProxySource `json:"proxy,omitzero"`
}

// StorageRef identifies Size bytes of a slot, Offset bytes from its low end.
type StorageRef struct {
	Slot   string `json:"slot"`
	Offset uint8  `json:"offset"`
	Size   uint8  `json:"size"`
}

// ProxySourceKind is the kind of a ProxySource.
type ProxySourceKind int

const (
	// ProxySourceAddress is a fixed implementation address in the code.
	ProxySourceAddress ProxySourceKind = iota
	// ProxySourceStorage is an address in one of the proxy's own storage slots.
	ProxySourceStorage
	// ProxySourceCall is an address another contract returns: a beacon, a registry.
	ProxySourceCall
	// ProxySourcePerSelector is a target that depends on the incoming selector.
	ProxySourcePerSelector
	// ProxySourceEip7702 is the delegate in an EIP-7702 designator.
	ProxySourceEip7702
)

// String returns the JSON type name for the source kind.
func (k ProxySourceKind) String() string {
	switch k {
	case ProxySourceAddress:
		return "Address"
	case ProxySourceStorage:
		return "Storage"
	case ProxySourceCall:
		return "Call"
	case ProxySourcePerSelector:
		return "PerSelector"
	case ProxySourceEip7702:
		return "Eip7702"
	default:
		return "Unknown"
	}
}

// ProxySource is a possible implementation source. Kind selects the field that is set:
// Address for ProxySourceAddress and ProxySourceEip7702, Storage for ProxySourceStorage,
// Call for ProxySourceCall, and none for ProxySourcePerSelector.
type ProxySource struct {
	Kind    ProxySourceKind
	Address string
	Storage *StorageRef
	Call    *ProxyCall
}

// UnmarshalJSON implements custom unmarshaling for ProxySource.
func (s *ProxySource) UnmarshalJSON(data []byte) error {
	var tagged struct {
		Type string          `json:"type"`
		Data json.RawMessage `json:"data,omitempty"`
	}
	if err := json.Unmarshal(data, &tagged); err != nil {
		return err
	}
	*s = ProxySource{}
	switch tagged.Type {
	case "Address":
		s.Kind = ProxySourceAddress
		return json.Unmarshal(tagged.Data, &s.Address)
	case "Eip7702":
		s.Kind = ProxySourceEip7702
		return json.Unmarshal(tagged.Data, &s.Address)
	case "Storage":
		s.Kind = ProxySourceStorage
		s.Storage = &StorageRef{}
		return json.Unmarshal(tagged.Data, s.Storage)
	case "Call":
		s.Kind = ProxySourceCall
		s.Call = &ProxyCall{}
		return json.Unmarshal(tagged.Data, s.Call)
	case "PerSelector":
		s.Kind = ProxySourcePerSelector
		return nil
	default:
		return fmt.Errorf("unknown proxy source type: %s", tagged.Type)
	}
}

// MarshalJSON implements custom marshaling for ProxySource with type and data fields.
func (s ProxySource) MarshalJSON() ([]byte, error) {
	var data any
	switch s.Kind {
	case ProxySourceAddress, ProxySourceEip7702:
		data = s.Address
	case ProxySourceStorage:
		data = s.Storage
	case ProxySourceCall:
		data = s.Call
	}
	return json.Marshal(struct {
		Type string `json:"type"`
		Data any    `json:"data,omitempty"`
	}{s.Kind.String(), data})
}

// ProxyCall describes an implementation lookup. A nil Selector means empty calldata.
type ProxyCall struct {
	Address  ProxyValue   `json:"address"`
	Selector *string      `json:"selector"`
	Args     []ProxyValue `json:"args"`
}

// ProxyValueKind is the kind of a ProxyValue.
type ProxyValueKind int

const (
	// ProxyValueConstant is a literal callee address or argument word.
	ProxyValueConstant ProxyValueKind = iota
	// ProxyValueStorage is bytes of a storage slot, in the word's low bytes.
	ProxyValueStorage
)

// String returns the JSON type name for the value kind.
func (k ProxyValueKind) String() string {
	switch k {
	case ProxyValueConstant:
		return "Constant"
	case ProxyValueStorage:
		return "Storage"
	default:
		return "Unknown"
	}
}

// ProxyValue is a lookup's callee address or one of its argument words. Kind selects the field
// that is set: Value for ProxyValueConstant, Storage for ProxyValueStorage.
type ProxyValue struct {
	Kind    ProxyValueKind
	Value   string
	Storage *StorageRef
}

// UnmarshalJSON implements custom unmarshaling for ProxyValue.
func (v *ProxyValue) UnmarshalJSON(data []byte) error {
	var tagged struct {
		Type string          `json:"type"`
		Data json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal(data, &tagged); err != nil {
		return err
	}
	*v = ProxyValue{}
	switch tagged.Type {
	case "Constant":
		v.Kind = ProxyValueConstant
		return json.Unmarshal(tagged.Data, &v.Value)
	case "Storage":
		v.Kind = ProxyValueStorage
		v.Storage = &StorageRef{}
		return json.Unmarshal(tagged.Data, v.Storage)
	default:
		return fmt.Errorf("unknown proxy value type: %s", tagged.Type)
	}
}

// MarshalJSON implements custom marshaling for ProxyValue with type and data fields.
func (v ProxyValue) MarshalJSON() ([]byte, error) {
	var data any = v.Value
	if v.Kind == ProxyValueStorage {
		data = v.Storage
	}
	return json.Marshal(struct {
		Type string `json:"type"`
		Data any    `json:"data"`
	}{v.Kind.String(), data})
}

// CborMetadata locates the terminal CBOR payload and exposes its text-keyed entries.
type CborMetadata struct {
	BytecodeOffset int         `json:"bytecodeOffset"`
	CborLength     int         `json:"cborLength"`
	Entries        []CborEntry `json:"entries"`
}

// CborEntry is a CBOR map entry having a text-string key.
type CborEntry struct {
	Key   string    `json:"key"`
	Value CborValue `json:"value"`
}

// CborValue is a decoded CBOR scalar or an exact encoded value.
type CborValue struct {
	Type  string          `json:"type"`
	Value json.RawMessage `json:"value"`
}

func (v CborValue) decode(expected string, target any) error {
	if v.Type != expected {
		return fmt.Errorf("CBOR value type is %q, not %s", v.Type, expected)
	}
	return json.Unmarshal(v.Value, target)
}

// AsString returns Value when Type is "string".
func (v CborValue) AsString() (string, error) {
	var value string
	return value, v.decode("string", &value)
}

// AsInteger returns Value when Type is "integer".
func (v CborValue) AsInteger() (int64, error) {
	var value int64
	return value, v.decode("integer", &value)
}

// AsBytes returns Value when Type is "bytes".
func (v CborValue) AsBytes() ([]byte, error) {
	var value []byte
	return value, v.decode("bytes", &value)
}

// AsBool returns Value when Type is "bool".
func (v CborValue) AsBool() (bool, error) {
	var value bool
	return value, v.decode("bool", &value)
}

// AsUndecoded returns the exact CBOR encoding when Type is "undecoded".
func (v CborValue) AsUndecoded() ([]byte, error) {
	var value []byte
	return value, v.decode("undecoded", &value)
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

// SelectorDispatch describes where a selector is dispatched in runtime bytecode.
type SelectorDispatch string

const (
	// SelectorDispatchABI indicates the normal external-function dispatcher.
	SelectorDispatchABI SelectorDispatch = "abi"
	// SelectorDispatchFallback indicates dispatch logic reached through the fallback path.
	SelectorDispatchFallback SelectorDispatch = "fallback"
)

// Function represents a selector-bearing smart contract entry point.
type Function struct {
	// Selector is the 4-byte function selector as hex string (e.g., "a9059cbb").
	Selector string `json:"selector"`
	// BytecodeOffset is the starting byte offset within EVM bytecode for the function body.
	BytecodeOffset int `json:"bytecode_offset"`
	// Dispatch identifies the selector's normal ABI or fallback dispatch path.
	Dispatch SelectorDispatch `json:"dispatch"`
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
	// ID is the unique block identifier (CFG key).
	ID int `json:"id"`
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
		ID    int             `json:"id"`
		Start int             `json:"start"`
		End   int             `json:"end"`
		Type  string          `json:"type"`
		Data  json.RawMessage `json:"data,omitempty"`
	}
	var alias blockAlias
	if err := json.Unmarshal(data, &alias); err != nil {
		return err
	}
	b.ID = alias.ID
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
		ID    int    `json:"id"`
		Start int    `json:"start"`
		End   int    `json:"end"`
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
		ID:    b.ID,
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
