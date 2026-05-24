# Stage 1: install workspace deps + build kitchen-sink.
# The native rivetkit-napi .node is pre-built outside this image and copied in
# via examples/kitchen-sink/rivetkit-napi.linux-x64-gnu.node (see
# .github/workflows/rivet-deploy-kitchen-sink.yml).
FROM node:22-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/*
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app
ENV NODE_OPTIONS=--max-old-space-size=7168
ENV SKIP_WASM_BUILD=1
ENV SKIP_NAPI_BUILD=1

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml turbo.json tsconfig.base.json tsup.base.ts ./
COPY patches/ ./patches/
COPY examples/kitchen-sink/ ./examples/kitchen-sink/
COPY rivetkit-typescript/packages/ ./rivetkit-typescript/packages/
COPY engine/sdks/typescript/ ./engine/sdks/typescript/
COPY shared/typescript/ ./shared/typescript/

# Move the pre-built napi .node into the workspace package so SKIP_NAPI_BUILD=1
# at runtime resolves via the local-file branch in
# rivetkit-typescript/packages/rivetkit-napi/index.js.
RUN mv examples/kitchen-sink/rivetkit-napi.linux-x64-gnu.node \
    rivetkit-typescript/packages/rivetkit-napi/rivetkit-napi.linux-x64-gnu.node

RUN pnpm install --frozen-lockfile
RUN mkdir -p rivetkit-openapi rivetkit-asyncapi
RUN SKIP_WASM_BUILD=1 SKIP_NAPI_BUILD=1 pnpm build --filter=kitchen-sink
RUN mkdir -p ./examples/kitchen-sink/dist/public

# Split the pnpm content-addressed store into multiple subdirs so the final
# image layers stay small enough for registry.rivet.dev to accept. A single
# /app/node_modules/.pnpm layer is ~300MB compressed and the registry 503s
# on push around that size.
RUN cd /app/node_modules/.pnpm \
    && mkdir -p /pnpm-chunks/c0 /pnpm-chunks/c1 /pnpm-chunks/c2 /pnpm-chunks/c3 \
       /pnpm-chunks/c4 /pnpm-chunks/c5 /pnpm-chunks/c6 /pnpm-chunks/c7 \
    && i=0 \
    && for d in */; do \
         mv "$d" "/pnpm-chunks/c$((i % 8))/$d"; \
         i=$((i + 1)); \
       done

# Stage 2: minimal runtime image.
# We deliberately copy the whole built workspace (sans heavyweight pnpm cache,
# which we re-layer separately for registry-friendly chunk sizes) instead of
# using `pnpm deploy`. Workspace packages live under
# /app/rivetkit-typescript/packages/* and are wired into /app/node_modules via
# pnpm symlinks; reproducing that layout in /deploy via `pnpm deploy` proved
# more trouble than it was worth.
FROM node:22-slim
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app
ENV NODE_ENV=production

# Source + workspace packages required by kitchen-sink at runtime.
COPY --from=builder /app/package.json /app/pnpm-workspace.yaml /app/turbo.json /app/tsconfig.base.json /app/tsup.base.ts ./
COPY --from=builder /app/examples/kitchen-sink/ ./examples/kitchen-sink/
COPY --from=builder /app/rivetkit-typescript/ ./rivetkit-typescript/
COPY --from=builder /app/engine/sdks/typescript/ ./engine/sdks/typescript/
COPY --from=builder /app/shared/typescript/ ./shared/typescript/

# node_modules tree with the symlink layout, minus the chunked .pnpm store.
COPY --from=builder /app/node_modules/ ./node_modules/

# Replay each pnpm CAS chunk back into .pnpm so all the symlinks resolve.
COPY --from=builder /pnpm-chunks/c0/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c1/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c2/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c3/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c4/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c5/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c6/ ./node_modules/.pnpm/
COPY --from=builder /pnpm-chunks/c7/ ./node_modules/.pnpm/

WORKDIR /app/examples/kitchen-sink
ENV PORT=8080
# Tell server.ts to enter serverless mode (serve /api/rivet/* via the registry
# handler, do NOT call registry.start()). The Rivet engine is provided by the
# managed pool; we only need this process to act as the runner.
# RIVET_RUN_ENGINE=1 cannot be used here because it would also start an
# embedded engine, conflicting with the cloud-supplied endpoint.
ENV KITCHEN_SINK_SERVERLESS_URL=cloud
EXPOSE 8080
# Run the bundled server directly so node is pid1. Signals reach the rivetkit
# SIGTERM handler without going through pnpm/tsx wrappers.
CMD ["node", "--enable-source-maps", "dist-server/server.mjs"]
