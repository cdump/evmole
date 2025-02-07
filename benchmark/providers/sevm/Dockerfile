# syntax=docker/dockerfile:1

FROM node:22
WORKDIR /app
RUN npm install sevm@0.7.4
COPY main.mjs /app
ENTRYPOINT ["node", "/app/main.mjs"]
