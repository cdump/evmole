# syntax=docker/dockerfile:1

FROM node:24
WORKDIR /app
RUN npm install @shazow/whatsabi@0.25.0
COPY main.mjs /app
ENTRYPOINT ["node", "/app/main.mjs"]
