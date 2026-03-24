# Build context: repository root
# Usage: docker build -f examples/sandbox/Dockerfile .

FROM node:22-slim AS build
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app

COPY . .
RUN --mount=type=cache,id=s/ddf210e7-d60c-433f-b2aa-e0915638c551-pnpm,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
RUN mkdir -p rivetkit-asyncapi rivetkit-openapi && \
    pnpm turbo build --filter=sandbox

# Production image.
FROM node:22-slim
WORKDIR /app
COPY --from=build /app/examples/sandbox/dist ./dist
COPY --from=build /app/examples/sandbox/public ./dist/public
# These packages use dynamic imports or require.resolve that bypass bundling.
RUN npm install @hono/node-server @hono/node-ws @rivetkit/sqlite srvx
EXPOSE 3000
CMD ["npx", "srvx", "--static", "public/", "dist/server.js"]
