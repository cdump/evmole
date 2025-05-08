#ifndef EVMOLE_H
#define EVMOLE_H
#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

/**
 * Configuration options for contract analysis.
 */
typedef struct {
    int selectors;            /* Include function selectors */
    int arguments;            /* Include function arguments */
    int state_mutability;     /* Include state mutability */
    int storage;              /* Include storage layout */
    int disassemble;          /* Include disassembled bytecode */
    int basic_blocks;         /* Include basic block analysis */
    int control_flow_graph;   /* Include control flow graph */
} EvmoleContractInfoOptions;

/**
 * Free memory allocated by this library.
 *
 * @param ptr Pointer to memory allocated by evmole
 */
void evmole_free(char* ptr);

/**
 * Analyzes contract bytecode and returns contract information in JSON format.
 *
 * @param code Runtime bytecode as a hex string
 * @param options Configuration options for the analysis
 * @param error_msg Pointer to store error message in case of failure
 * @return JSON string containing analysis results or NULL on error
 *         (memory must be freed with evmole_free)
 */
char* evmole_contract_info(
    const char* code,
    EvmoleContractInfoOptions options,
    char** error_msg
);

#ifdef __cplusplus
} /* extern \"C\" */
#endif
#endif /* EVMOLE_H */
