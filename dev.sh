#!/bin/sh

DS=$2

BDIR=`pwd`/benchmark

case $1 in
    js)
        ln -s `pwd`/js ${BDIR}/providers/evmole-js 2>/dev/null || true
        node ${BDIR}/providers/evmole-js/main.mjs \
            arguments \
            ${BDIR}/datasets/${2} \
            out.json \
            ${BDIR}/results/etherscan.selectors_${2}.json \
            --filter-filename ${3} \
            --filter-selector ${4}
    ;;

    rs)
        ln -s `pwd`/rust ${BDIR}/providers/evmole-rs 2>/dev/null || true
        cargo run \
            --manifest-path benchmark/providers/evmole-rs/Cargo.toml \
            --features "evmole/trace" \
            arguments \
            ${BDIR}/datasets/${2} \
            out.json \
            ${BDIR}/results/etherscan.selectors_${2}.json \
            --filter-filename ${3} \
            --filter-selector ${4}
    ;;

    py)
        PYTHONPATH=`pwd` \
            python3.12 \
            ${BDIR}/providers/evmole-py/main.py \
            arguments \
            ${BDIR}/datasets/${2} \
            out.json \
            ${BDIR}/results/etherscan.selectors_${2}.json \
            --filter-filename ${3} \
            --filter-selector ${4}
    ;;

    *)
        echo 'unknown "$1"'
        exit 1;
esac
