package evmole_test

import (
	_ "embed"
	"encoding/hex"
	"slices"
	"testing"

	evmole "github.com/cdump/evmole/go"
)

// This is the bytecode of a Uniswapv3 pool
// @see https://etherscan.io/address/0x5777d92f208679DB4b9778590Fa3CAB3aC9e2168#code
//
//go:embed uniswapv3.bytecode
var uniswapv3Bytecode string

// TestContractInfo tests the evmole.Analyze function on the Uniswapv3 pool contract
// It ensures all the read methods are present in the result
func TestContractInfo(t *testing.T) {
	bytecode, err := hex.DecodeString(uniswapv3Bytecode)
	if err != nil {
		t.Fatalf("Error: %v", err)
	}

	result, err := evmole.Analyze(bytecode)
	if err != nil {
		t.Fatalf("Error: %v", err)
	}

	expectedMethods := []string{
		"c45a0155", // factory
		"ddca3f43", // fee
		"f3058399", // feeGrowthGlobal0X128
		"46141319", // feeGrowthGlobal1X128
		"1a686502", // liquidity
		"70cf754a", // maxLiquidityPerTick
		"252c09d7", // observations
		"883bdbfd", // observe
		"514ea4bf", // positions
		"1ad8b03b", // protocolFees
		"3850c7bd", // slot0
		"a38807f2", // snapshotCumulativesInside
		"5339c296", // tickBitmap
		"d0c93a7c", // tickSpacing
		"f30dba93", // ticks
		"0dfe1681", // token0
		"d21220a7", // token1
	}
	for _, method := range expectedMethods {
		found := false
		for _, f := range result.Functions {
			if f.Selector == method {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Expected method %s not found", method)
		}
	}

	for _, method := range expectedMethods {
		for _, s := range result.Storage {
			if slices.Contains(s.Reads, method) {
				t.Logf("Storage %s reads %s (offset: %d, type: %s)", s.Slot, method, s.Offset, s.Type)
			}
		}
	}
}
