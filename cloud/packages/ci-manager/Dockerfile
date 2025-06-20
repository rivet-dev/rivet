# Build stage - includes dev dependencies
FROM node:22-alpine AS builder
ENV NODE_ENV=development

WORKDIR /app
RUN npm i -g corepack && corepack enable
COPY package.json yarn.lock ./
RUN yarn install --immutable
COPY . .
RUN yarn build

# Production stage - only prod dependencies
FROM node:22-alpine AS release
ENV NODE_ENV=production

RUN adduser -s /bin/sh -D rivet
WORKDIR /app
RUN apk add --no-cache skopeo umoci && npm i -g corepack && corepack enable
COPY package.json yarn.lock ./
RUN yarn config set nodeLinker node-modules && yarn install --immutable
COPY --from=builder /app/dist ./dist
RUN chown -R rivet:rivet /app
USER rivet
EXPOSE 3000
CMD ["node", "dist/index.js"]
