# syntax=docker/dockerfile:1

ARG GIGAHORSE_COMMIT=1b06cef82f0a29790e87f576eba6d6d793170d70
ARG SOUFFLE_VERSION=2.4.1
ARG SOUFFLE_COMMIT=01f11777b4b09329b8232466d82376e039ac1ba8

# Upstream's published images still date from 2024. Rebuild its dependencies on a
# fresh Ubuntu image, following docker/dependencies/souffle24/Dockerfile at GIGAHORSE_COMMIT:
# https://github.com/nevillegrech/gigahorse-toolchain
FROM ubuntu:22.04 AS setup
COPY --from=ghcr.io/astral-sh/uv:0.13 /uv /usr/local/bin/uv
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        bison build-essential ca-certificates cmake flex git libboost-all-dev \
        libffi-dev libncurses-dev libsqlite3-dev libssl-dev libz3-dev mcpp \
        util-linux z3 zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*
# Match upstream's Souffle build; discard its source and build tree in this layer.
ARG SOUFFLE_VERSION
ARG SOUFFLE_COMMIT
RUN git clone --depth 1 --branch "$SOUFFLE_VERSION" https://github.com/souffle-lang/souffle.git /tmp/souffle \
    && test "$(git -C /tmp/souffle rev-parse HEAD)" = "$SOUFFLE_COMMIT" \
    && cmake -S /tmp/souffle -B /tmp/souffle/build -DCMAKE_INSTALL_PREFIX=/usr/local -DSOUFFLE_ENABLE_TESTING=OFF \
    && cmake --build /tmp/souffle/build --target install -j 2 \
    && rm -rf /tmp/souffle \
    && souffle --version
ENV UV_PYTHON_INSTALL_DIR=/opt/uv/python \
    UV_PYTHON_PREFERENCE=only-managed \
    UV_LINK_MODE=copy \
    PATH="/gigahorse/.venv/bin:${PATH}"
WORKDIR /gigahorse
ARG GIGAHORSE_COMMIT
RUN git clone https://github.com/nevillegrech/gigahorse-toolchain.git . \
    && git checkout "$GIGAHORSE_COMMIT" \
    && git submodule update --init --recursive \
    && uv sync --frozen --no-dev \
    && make -C souffle-addon libsoufflenum.so \
    && python3 gigahorse.py --help > /dev/null \
    && uv cache clean
# Test dependencies and Python setup without the large Datalog compilation:
# docker build --target setup .
FROM setup AS benchmark
COPY storage_layout.dl clients/
# Compile every Datalog program now; benchmark runs reuse the cached binaries.
# One at a time: gigahorse compiles them in parallel, and even the storage client alone
# needs more RAM than a 8GB host has free (add swap there)
RUN printf '#!/bin/sh\nexec flock /tmp/souffle.lock souffle "$@"\n' > /tmp/souffle-serial \
    && chmod +x /tmp/souffle-serial \
    && echo 0x00 > /tmp/warmup.hex \
    && python3 gigahorse.py -j 1 -S /tmp/souffle-serial -C clients/storage_layout.dl -w /tmp/warmup -r /tmp/warmup.json /tmp/warmup.hex \
    && ls cache/storage_layout.dl_compiled \
    && rm -rf /tmp/warmup* /tmp/souffle-serial
WORKDIR /app
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
