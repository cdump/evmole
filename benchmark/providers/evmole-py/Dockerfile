# syntax=docker/dockerfile:1

FROM rust:1.79 AS build
RUN  wget https://github.com/PyO3/maturin/releases/download/v1.7.0/maturin-musl_1.7.0_amd64.deb \
  && apt-get install ./maturin-*.deb
COPY ./rust /workdir
WORKDIR /workdir
RUN maturin build --release --out wheel/

FROM python:3.11-slim
WORKDIR /app
COPY --from=build /workdir/wheel ./wheel
RUN pip3 install ./wheel/*.whl
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
