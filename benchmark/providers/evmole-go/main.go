package main

import (
	"context"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/cdump/evmole/go"
)

type Input struct {
	Code            string `json:"code"`
	RuntimeBytecode string `json:"runtimeBytecode"`
}

func main() {
	if len(os.Args) < 4 {
		fmt.Println("Usage: main MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]")
		os.Exit(1)
	}

	mode := os.Args[1]
	inputDir := os.Args[2]
	outputFile := os.Args[3]

	var selectors map[string][]any
	if mode == "arguments" || mode == "mutability" {
		data, err := os.ReadFile(os.Args[4])
		if err != nil {
			panic(err)
		}
		if err := json.Unmarshal(data, &selectors); err != nil {
			panic(err)
		}
	}

	ctx := context.Background()
	analyzer, err := evmole.NewAnalyzer(ctx)
	if err != nil {
		panic(err)
	}
	defer analyzer.Close(ctx)

	retSelectors := make(map[string][]any)
	retOther := make(map[string][]any)
	retFlow := make(map[string][]any)

	entries, err := os.ReadDir(inputDir)
	if err != nil {
		panic(err)
	}

	for _, entry := range entries {
		fname := entry.Name()
		fpath := filepath.Join(inputDir, fname)

		data, err := os.ReadFile(fpath)
		if err != nil {
			panic(err)
		}

		var input Input
		if err := json.Unmarshal(data, &input); err != nil {
			panic(err)
		}

		codeStr := input.Code
		if input.RuntimeBytecode != "" {
			codeStr = input.RuntimeBytecode
		}
		codeStr = strings.TrimPrefix(codeStr, "0x")
		code, err := hex.DecodeString(codeStr)
		if err != nil {
			panic(err)
		}

		switch mode {
		case "selectors":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{Selectors: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			sels := make([]string, len(info.Functions))
			for i, f := range info.Functions {
				sels[i] = f.Selector
			}
			retSelectors[fname] = []any{durationUS, sels}

		case "arguments":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{Arguments: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			bySel := make(map[string]string)
			for _, f := range info.Functions {
				if f.Arguments != nil {
					bySel[f.Selector] = *f.Arguments
				}
			}
			fsel := selectors[fname][1].([]any)
			res := make(map[string]string)
			for _, s := range fsel {
				sel := s.(string)
				if v, ok := bySel[sel]; ok {
					res[sel] = v
				} else {
					res[sel] = "notfound"
				}
			}
			retOther[fname] = []any{durationUS, res}

		case "mutability":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{StateMutability: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			bySel := make(map[string]string)
			for _, f := range info.Functions {
				if f.StateMutability != nil {
					bySel[f.Selector] = *f.StateMutability
				}
			}
			fsel := selectors[fname][1].([]any)
			res := make(map[string]string)
			for _, s := range fsel {
				sel := s.(string)
				if v, ok := bySel[sel]; ok {
					res[sel] = v
				} else {
					res[sel] = "notfound"
				}
			}
			retOther[fname] = []any{durationUS, res}

		case "storage":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{Storage: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			res := make(map[string]string)
			for _, sr := range info.Storage {
				key := fmt.Sprintf("%s_%d", sr.Slot, sr.Offset)
				res[key] = sr.Type
			}
			retOther[fname] = []any{durationUS, res}

		case "blocks":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{BasicBlocks: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			blocks := make([][2]int, len(info.BasicBlocks))
			for i, b := range info.BasicBlocks {
				blocks[i] = [2]int{b.Start, b.End}
			}
			retFlow[fname] = []any{durationUS, blocks}

		case "flow":
			start := time.Now()
			info, err := analyzer.ContractInfo(ctx, code, evmole.Options{ControlFlowGraph: true})
			durationUS := time.Since(start).Microseconds()
			if err != nil {
				panic(err)
			}
			edges := make([][2]int, 0)
			for _, block := range info.ControlFlowGraph.Blocks {
				switch block.Type.Kind {
				case evmole.BlockKindJump:
					edges = append(edges, [2]int{block.Start, block.Type.Jump.To})
				case evmole.BlockKindJumpi:
					edges = append(edges, [2]int{block.Start, block.Type.Jumpi.TrueTo})
					edges = append(edges, [2]int{block.Start, block.Type.Jumpi.FalseTo})
				case evmole.BlockKindDynamicJump:
					for _, v := range block.Type.DynamicJump.To {
						if v.To != nil {
							edges = append(edges, [2]int{block.Start, *v.To})
						}
					}
				case evmole.BlockKindDynamicJumpi:
					for _, v := range block.Type.DynamicJumpi.TrueTo {
						if v.To != nil {
							edges = append(edges, [2]int{block.Start, *v.To})
						}
					}
					edges = append(edges, [2]int{block.Start, block.Type.DynamicJumpi.FalseTo})
				case evmole.BlockKindTerminate:
					// do nothing
				}
			}
			retFlow[fname] = []any{durationUS, edges}

		default:
			panic(fmt.Sprintf("unknown mode: %s", mode))
		}
	}

	var result any
	switch mode {
	case "selectors":
		result = retSelectors
	case "blocks", "flow":
		result = retFlow
	default:
		result = retOther
	}

	out, err := json.Marshal(result)
	if err != nil {
		panic(err)
	}
	if err := os.WriteFile(outputFile, out, 0o644); err != nil {
		panic(err)
	}
}
