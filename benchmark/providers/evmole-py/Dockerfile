# syntax=docker/dockerfile:1

FROM rust:1.84 AS build
RUN  wget https://github.com/PyO3/maturin/releases/download/v1.8.1/maturin-x86_64-unknown-linux-musl.tar.gz \
  && tar xf maturin-*.tar.gz && mv maturin /usr/local/bin/
COPY ./rust /workdir
WORKDIR /workdir
RUN maturin build --release --out wheel/

FROM python:3.11-slim
WORKDIR /app
COPY --from=build /workdir/wheel ./wheel
RUN pip3 install ./wheel/*.whl
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
