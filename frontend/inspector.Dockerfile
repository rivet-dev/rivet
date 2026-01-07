# Frontend (Inspector) Dockerfile
# Multi-stage build: Node.js for building, Caddy for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM node:22-alpine AS builder

# Install git, git-lfs, and coreutils (for env -S support in build scripts)
RUN apk add --no-cache git git-lfs coreutils

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml tsconfig.base.json turbo.json tsup.base.ts ./

# Copy all workspace packages that frontend depends on (including transitive deps)
# frontend -> rivetkit, @rivetkit/engine-api-full
# rivetkit -> @rivetkit/virtual-websocket, @rivetkit/engine-runner
# @rivetkit/engine-runner -> @rivetkit/engine-runner-protocol
COPY frontend/ frontend/
COPY engine/sdks/typescript/api-full/ engine/sdks/typescript/api-full/
COPY engine/sdks/typescript/runner/ engine/sdks/typescript/runner/
COPY engine/sdks/typescript/runner-protocol/ engine/sdks/typescript/runner-protocol/
COPY rivetkit-typescript/packages/rivetkit/ rivetkit-typescript/packages/rivetkit/
COPY shared/typescript/virtual-websocket/ shared/typescript/virtual-websocket/

# Copy generated API docs (used by rivetkit build)
COPY rivetkit-asyncapi/ rivetkit-asyncapi/
COPY rivetkit-openapi/ rivetkit-openapi/

# Fetch LFS files if build platform doesn't support Git LFS natively
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

# Install dependencies (with pnpm store cache)
RUN --mount=type=cache,id=s/11ac71ef-9b68-4d4c-bc8a-bc8b45000c14-/pnpm/store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# Build arguments for environment variables
# Use placeholder URLs that pass validation but can be replaced at runtime
ARG VITE_APP_API_URL="https://VITE_APP_API_URL.placeholder.rivet.gg"
ARG VITE_APP_ASSETS_URL="https://VITE_APP_ASSETS_URL.placeholder.rivet.gg"
ARG VITE_APP_SENTRY_DSN="https://VITE_APP_SENTRY_DSN.placeholder.rivet.gg/0"
ARG VITE_APP_SENTRY_PROJECT_ID="0"
ARG VITE_APP_POSTHOG_API_KEY=""
ARG VITE_APP_POSTHOG_HOST=""
ARG DEPLOYMENT_TYPE="staging"
ARG FONTAWESOME_PACKAGE_TOKEN=""

# Set environment variables for build
ENV VITE_APP_API_URL=${VITE_APP_API_URL}
ENV VITE_APP_ASSETS_URL=${VITE_APP_ASSETS_URL}
ENV VITE_APP_SENTRY_DSN=${VITE_APP_SENTRY_DSN}
ENV VITE_APP_SENTRY_PROJECT_ID=${VITE_APP_SENTRY_PROJECT_ID}
ENV VITE_APP_POSTHOG_API_KEY=${VITE_APP_POSTHOG_API_KEY}
ENV VITE_APP_POSTHOG_HOST=${VITE_APP_POSTHOG_HOST}
ENV DEPLOYMENT_TYPE=${DEPLOYMENT_TYPE}
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

# Build the inspector frontend using turbo (automatically builds all dependencies, with turbo cache)
RUN --mount=type=cache,id=s/11ac71ef-9b68-4d4c-bc8a-bc8b45000c14-/app/.turbo,target=/app/.turbo \
    npx turbo run build:inspector --filter=@rivetkit/engine-frontend

# =============================================================================
# Stage 2: Serve with Caddy
# =============================================================================
FROM caddy:alpine

# Install bash for entrypoint script
RUN apk add --no-cache bash

# Copy Caddyfile configuration
COPY frontend/Caddyfile /etc/caddy/Caddyfile

# Copy built files from builder stage
COPY --from=builder /app/frontend/dist /srv

# Copy entrypoint script for runtime env var substitution
COPY frontend/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

# Default port (platform injects PORT env var)
ENV PORT=80

# Use custom entrypoint for env var substitution
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
