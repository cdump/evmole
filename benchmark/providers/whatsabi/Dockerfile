# syntax=docker/dockerfile:1

FROM node:22
WORKDIR /app
RUN npm install @shazow/whatsabi@0.11.0
COPY main.mjs /app
ENTRYPOINT ["node", "/app/main.mjs"]
