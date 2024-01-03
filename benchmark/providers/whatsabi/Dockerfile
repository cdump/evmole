FROM node:21
WORKDIR /app
RUN npm install @shazow/whatsabi@0.10.0
COPY main.mjs /app
ENTRYPOINT ["node", "/app/main.mjs"]
