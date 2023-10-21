FROM node:20.8-slim

EXPOSE 5051

WORKDIR /app

COPY package*.json ./
RUN npm install
RUN npm ci --omit=dev

COPY . .
CMD [ "node", "index.js" ]
