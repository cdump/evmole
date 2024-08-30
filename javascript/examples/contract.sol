contract Contract {
    function a(uint32 v) public pure returns (uint32) {
        return v + 1;
    }
}

// Runtime binary:
// $ solc --no-cbor-metadata --optimize --hashes --bin-runtime ./contract.sol
