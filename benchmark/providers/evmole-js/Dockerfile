# syntax=docker/dockerfile:1

FROM node:22 AS build
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.84
COPY ./rust /workdir
WORKDIR /workdir/javascript
ENV PATH=/root/.cargo/bin:$PATH
RUN npm ci && npm run build && npm pack

FROM node:22
WORKDIR /app
COPY --from=build /workdir/javascript/evmole-*.tgz ./
RUN npm install ./evmole-*.tgz
COPY main.mjs /app
ENTRYPOINT ["node", "/app/main.mjs"]
