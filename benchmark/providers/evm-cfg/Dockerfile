# syntax=docker/dockerfile:1

FROM rust:1.87 as build
RUN cargo install --git https://github.com/plotchy/evm-cfg --root /installed

FROM python:3.12-slim
WORKDIR /app
RUN pip3 install pydot==3.0.4
COPY --from=build /installed/bin/evm-cfg /usr/bin/evm-cfg
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
