# syntax=docker/dockerfile:1

FROM python:3.12-slim
RUN pip install evm-cfg-builder==0.3.1
WORKDIR /app
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
