package evmole

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"testing"
)

const testCodeHex = "6080604052348015600e575f80fd5b50600436106026575f3560e01c8063fae7ab8214602a575b5f80fd5b603960353660046062565b6052565b60405163ffffffff909116815260200160405180910390f35b5f605c826001608a565b92915050565b5f602082840312156071575f80fd5b813563ffffffff811681146083575f80fd5b9392505050565b63ffffffff8181168382160190811115605c57634e487b7160e01b5f52601160045260245ffd"

var testCode, _ = hex.DecodeString(testCodeHex)

func TestContractInfoBasic(t *testing.T) {
	ctx := context.Background()

	info, err := ContractInfo(ctx, testCode, Options{
		Selectors:       true,
		Arguments:       true,
		StateMutability: true,
	})
	if err != nil {
		t.Fatalf("ContractInfo failed: %v", err)
	}

	if len(info.Functions) == 0 {
		t.Fatal("expected at least one function")
	}

	fn := info.Functions[0]
	if fn.Selector != "fae7ab82" {
		t.Errorf("expected selector 'fae7ab82', got '%s'", fn.Selector)
	}
	if fn.Arguments == nil || *fn.Arguments != "uint32" {
		args := "<nil>"
		if fn.Arguments != nil {
			args = *fn.Arguments
		}
		t.Errorf("expected arguments 'uint32', got '%s'", args)
	}
	if fn.StateMutability == nil || *fn.StateMutability != "pure" {
		sm := "<nil>"
		if fn.StateMutability != nil {
			sm = *fn.StateMutability
		}
		t.Errorf("expected state mutability 'pure', got '%s'", sm)
	}
}

func TestContractInfoDisassemble(t *testing.T) {
	ctx := context.Background()

	info, err := ContractInfo(ctx, testCode, Options{
		Disassemble: true,
	})
	if err != nil {
		t.Fatalf("ContractInfo failed: %v", err)
	}

	if len(info.Disassembled) == 0 {
		t.Fatal("expected disassembled opcodes")
	}

	firstOp := info.Disassembled[0]
	if firstOp.Offset != 0 || firstOp.Opcode != "PUSH1 80" {
		t.Errorf("expected first opcode (0, 'PUSH1 80'), got (%d, '%s')", firstOp.Offset, firstOp.Opcode)
	}
}

func TestContractInfoControlFlowGraph(t *testing.T) {
	ctx := context.Background()

	info, err := ContractInfo(ctx, testCode, Options{
		BasicBlocks:      true,
		ControlFlowGraph: true,
		Selectors:        true,
	})
	if err != nil {
		t.Fatalf("ContractInfo failed: %v", err)
	}

	if len(info.BasicBlocks) == 0 {
		t.Fatal("expected basic blocks")
	}

	if info.ControlFlowGraph == nil {
		t.Fatal("expected control flow graph")
	}

	if len(info.ControlFlowGraph.Blocks) == 0 {
		t.Fatal("expected blocks in control flow graph")
	}

	// First block should be a Jumpi
	block := info.ControlFlowGraph.Blocks[0]
	if block.Type.Kind != BlockKindJumpi {
		t.Errorf("expected first block to be Jumpi, got kind %d", block.Type.Kind)
	}
	if block.Type.Jumpi == nil {
		t.Fatal("expected Jumpi data")
	}
}

func TestAnalyzerReuse(t *testing.T) {
	ctx := context.Background()

	analyzer, err := NewAnalyzer(ctx)
	if err != nil {
		t.Fatalf("NewAnalyzer failed: %v", err)
	}
	defer analyzer.Close(ctx)

	// Call multiple times to verify reuse works
	for i := range 3 {
		info, err := analyzer.ContractInfo(ctx, testCode, Options{
			Selectors: true,
		})
		if err != nil {
			t.Fatalf("ContractInfo (iteration %d) failed: %v", i, err)
		}
		if len(info.Functions) != 1 {
			t.Errorf("iteration %d: expected 1 function, got %d", i, len(info.Functions))
		}
	}
}

func TestBlockUnmarshalUnknownType(t *testing.T) {
	jsonStr := `{"start": 0, "end": 10, "type": "UnknownType", "data": {}}`
	var b Block
	err := json.Unmarshal([]byte(jsonStr), &b)
	if err == nil {
		t.Fatal("Expected error for unknown block type, got nil")
	}
	t.Logf("Got expected error: %v", err)
}

func TestStorageDetection(t *testing.T) {
	ctx := context.Background()
	// Minimal bytecode: PUSH1 0x01, PUSH1 0x00, SSTORE (Store 1 at slot 0)
	// Hex: 6001600055
	// Note: evmole usually expects a dispatcher or at least valid CFG.
	// This is valid CFG (one block).
	code, _ := hex.DecodeString("6001600055")

	info, err := ContractInfo(ctx, code, Options{
		Storage: true,
	})
	if err != nil {
		t.Fatalf("ContractInfo failed: %v", err)
	}

	// This specific bytecode might not trigger 'Storage' detection if evmole expects
	// the storage access to be part of a function reachable via selector.
	// But let's see. If it fails to find it, we know we need a better test case.
	// For now, we just want to ensure it doesn't crash and returns a result.
	if info.Storage == nil {
		t.Log("Storage is nil for simple bytecode")
	} else {
		t.Logf("Storage found: %d entries", len(info.Storage))
		for _, s := range info.Storage {
			t.Logf("  Slot: %s, Type: %s", s.Slot, s.Type)
		}
	}
}

func TestJSONRoundTrip(t *testing.T) {
	ctx := context.Background()

	info, err := ContractInfo(ctx, testCode, Options{
		Selectors:        true,
		Arguments:        true,
		StateMutability:  true,
		Disassemble:      true,
		BasicBlocks:      true,
		ControlFlowGraph: true,
	})
	if err != nil {
		t.Fatalf("ContractInfo failed: %v", err)
	}

	// Serialize to JSON
	data, err := json.Marshal(info)
	if err != nil {
		t.Fatalf("json.Marshal failed: %v", err)
	}

	// Deserialize back
	var info2 Contract
	if err := json.Unmarshal(data, &info2); err != nil {
		t.Fatalf("json.Unmarshal failed: %v", err)
	}

	// Verify functions
	if len(info2.Functions) != len(info.Functions) {
		t.Errorf("Functions count mismatch: got %d, want %d", len(info2.Functions), len(info.Functions))
	}
	if len(info.Functions) > 0 && len(info2.Functions) > 0 {
		if info2.Functions[0].Selector != info.Functions[0].Selector {
			t.Errorf("Function selector mismatch: got %s, want %s", info2.Functions[0].Selector, info.Functions[0].Selector)
		}
	}

	// Verify disassembled
	if len(info2.Disassembled) != len(info.Disassembled) {
		t.Errorf("Disassembled count mismatch: got %d, want %d", len(info2.Disassembled), len(info.Disassembled))
	}
	if len(info.Disassembled) > 0 && len(info2.Disassembled) > 0 {
		if info2.Disassembled[0] != info.Disassembled[0] {
			t.Errorf("First instruction mismatch: got %+v, want %+v", info2.Disassembled[0], info.Disassembled[0])
		}
	}

	// Verify basic blocks
	if len(info2.BasicBlocks) != len(info.BasicBlocks) {
		t.Errorf("BasicBlocks count mismatch: got %d, want %d", len(info2.BasicBlocks), len(info.BasicBlocks))
	}

	// Verify control flow graph
	if info.ControlFlowGraph == nil || info2.ControlFlowGraph == nil {
		t.Fatal("ControlFlowGraph is nil")
	}
	if len(info2.ControlFlowGraph.Blocks) != len(info.ControlFlowGraph.Blocks) {
		t.Errorf("CFG blocks count mismatch: got %d, want %d", len(info2.ControlFlowGraph.Blocks), len(info.ControlFlowGraph.Blocks))
	}
	if len(info.ControlFlowGraph.Blocks) > 0 && len(info2.ControlFlowGraph.Blocks) > 0 {
		b1 := info.ControlFlowGraph.Blocks[0]
		b2 := info2.ControlFlowGraph.Blocks[0]
		if b2.Start != b1.Start || b2.End != b1.End {
			t.Errorf("First block mismatch: got {%d,%d}, want {%d,%d}", b2.Start, b2.End, b1.Start, b1.End)
		}
		if b2.Type.Kind != b1.Type.Kind {
			t.Errorf("First block type mismatch: got %v, want %v", b2.Type.Kind, b1.Type.Kind)
		}
	}
}

func BenchmarkAnalyzerCreation(b *testing.B) {
	ctx := context.Background()
	for b.Loop() {
		analyzer, err := NewAnalyzer(ctx)
		if err != nil {
			b.Fatal(err)
		}
		analyzer.Close(ctx)
	}
}

func BenchmarkContractInfo(b *testing.B) {
	ctx := context.Background()

	analyzer, err := NewAnalyzer(ctx)
	if err != nil {
		b.Fatal(err)
	}
	defer analyzer.Close(ctx)

	b.ResetTimer()
	for b.Loop() {
		_, err := analyzer.ContractInfo(ctx, testCode, Options{
			Selectors:       true,
			Arguments:       true,
			StateMutability: true,
		})
		if err != nil {
			b.Fatal(err)
		}
	}
}
