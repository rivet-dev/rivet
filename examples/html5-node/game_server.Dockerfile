FROM node:20-alpine
RUN adduser -D server
WORKDIR /app

COPY package.json package.json
COPY packages/rivet-sdk ./packages/rivet-sdk
COPY packages/game-server ./packages/game-server
WORKDIR /app/packages/game-server
RUN npm install --frozen-lockfile && npm run build

RUN chown -R server:server /app
USER server
EXPOSE 7777

CMD ["node", "dist/index.js"]
