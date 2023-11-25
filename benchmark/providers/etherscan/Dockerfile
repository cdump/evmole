FROM python:3.12-slim
WORKDIR /app
RUN pip3 install pycryptodome==3.19
COPY main.py /app
ENTRYPOINT ["python3", "/app/main.py"]
