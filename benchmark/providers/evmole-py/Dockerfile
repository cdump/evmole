# syntax=docker/dockerfile:1

FROM python:3.12-slim
WORKDIR /app
COPY main.py /app
COPY evmole /app/evmole
ENTRYPOINT ["python3", "/app/main.py"]
