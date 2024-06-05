# syntax=docker/dockerfile:1

FROM node:22
WORKDIR /app
COPY main.mjs /app
COPY js /app/js
ENTRYPOINT ["node", "/app/main.mjs"]
