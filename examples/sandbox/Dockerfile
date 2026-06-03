# Compatibility Dockerfile for the Railway kitchen-sink preview service.
# The Railway service still points at this historical path, so build the real kitchen-sink image from here.
FROM node:22-slim
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app
ENV NODE_OPTIONS=--max-old-space-size=7168

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml turbo.json tsconfig.base.json tsup.base.ts ./
COPY patches/ ./patches/
COPY examples/kitchen-sink/ ./examples/kitchen-sink/
COPY rivetkit-typescript/packages/ ./rivetkit-typescript/packages/
COPY engine/sdks/typescript/ ./engine/sdks/typescript/
COPY shared/typescript/ ./shared/typescript/

RUN pnpm install --frozen-lockfile
RUN mkdir -p rivetkit-openapi rivetkit-asyncapi
RUN pnpm build --filter=kitchen-sink

WORKDIR /app/examples/kitchen-sink

EXPOSE 8080
CMD ["pnpm", "start"]
