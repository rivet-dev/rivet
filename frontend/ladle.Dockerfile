# Frontend (Ladle) Dockerfile
FROM node:22-alpine AS builder

RUN apk add --no-cache git git-lfs coreutils

RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml tsconfig.base.json turbo.json tsup.base.ts ./

# Copy frontend package
COPY frontend/ frontend/

# Copy public examples (required by Vite during ladle build)
COPY frontend/public/examples/ frontend/public/examples/

# Copy engine SDK dependencies
COPY engine/sdks/typescript/api-full/ engine/sdks/typescript/api-full/
COPY engine/sdks/typescript/runner/ engine/sdks/typescript/runner/
COPY engine/sdks/typescript/runner-protocol/ engine/sdks/typescript/runner-protocol/

# Copy rivetkit dependencies
COPY rivetkit-typescript/packages/rivetkit/ rivetkit-typescript/packages/rivetkit/
COPY rivetkit-typescript/packages/traces/ rivetkit-typescript/packages/traces/
COPY rivetkit-typescript/packages/workflow-engine/ rivetkit-typescript/packages/workflow-engine/
COPY rivetkit-typescript/packages/sqlite-vfs/ rivetkit-typescript/packages/sqlite-vfs/
COPY rivetkit-typescript/packages/sqlite-vfs-linux-arm64/ rivetkit-typescript/packages/sqlite-vfs-linux-arm64/
COPY rivetkit-typescript/packages/sqlite-vfs-linux-x64/ rivetkit-typescript/packages/sqlite-vfs-linux-x64/

# Copy shared libraries
COPY shared/typescript/virtual-websocket/ shared/typescript/virtual-websocket/

# Copy examples (needed for ladle stories)
COPY examples/ examples/

# Copy generated API docs
COPY rivetkit-asyncapi/ rivetkit-asyncapi/
COPY rivetkit-openapi/ rivetkit-openapi/

# Fetch LFS files
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

RUN --mount=type=cache,id=s/465998c9-9dc0-4af4-ac91-b772d7596d6e-pnpm-store,target=/pnpm/store \
    pnpm install --frozen-lockfile

RUN --mount=type=cache,id=s/465998c9-9dc0-4af4-ac91-b772d7596d6e-turbo,target=/app/.turbo \
    npx turbo run build:ladle --filter=@rivetkit/engine-frontend

FROM caddy:alpine

COPY frontend/Caddyfile.ladle /etc/caddy/Caddyfile
COPY --from=builder /app/frontend/build /srv

ENV PORT=80
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
