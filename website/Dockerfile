# Website Astro Dockerfile
# Multi-stage build: Node.js for building, Caddy for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
# Use Debian-based Node image (required for Playwright/mermaid rendering)
FROM node:22-bookworm AS builder

# Install git-lfs and Playwright browser dependencies
RUN apt-get update && apt-get install -y git-lfs && rm -rf /var/lib/apt/lists/*
RUN npx -y playwright install --with-deps chromium

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml turbo.json tsconfig.base.json tsup.base.ts ./

# Copy all workspace packages that website depends on
COPY website/ website/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/example-registry/ frontend/packages/example-registry/
COPY examples/ examples/
COPY engine/artifacts/ engine/artifacts/
COPY engine/sdks/typescript/ engine/sdks/typescript/
COPY rivetkit-typescript/artifacts/ rivetkit-typescript/artifacts/
COPY rivetkit-typescript/packages/ rivetkit-typescript/packages/
COPY rivetkit-openapi/ rivetkit-openapi/
COPY rivetkit-asyncapi/ rivetkit-asyncapi/
COPY shared/typescript/ shared/typescript/

# Fetch LFS files if build platform doesn't support Git LFS natively
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

# Arguments required before installing dependencies
ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

# Install dependencies (with pnpm store cache)
RUN --mount=type=cache,id=s/3e31e381-166e-4a85-983a-a49830a88b96-/pnpm/store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# Re-install Playwright browsers for the correct version (pnpm may install a different version)
RUN pnpm exec playwright install chromium

# Build arguments for PUBLIC_* environment variables
ARG PUBLIC_SITE_URL="https://rivet.gg"
ARG PUBLIC_POSTHOG_KEY=""
ARG PUBLIC_POSTHOG_HOST=""
ARG PUBLIC_TYPESENSE_HOST=""
ARG PUBLIC_TYPESENSE_API_KEY=""

# Set environment variables for build
ENV PUBLIC_SITE_URL=${PUBLIC_SITE_URL}
ENV PUBLIC_POSTHOG_KEY=${PUBLIC_POSTHOG_KEY}
ENV PUBLIC_POSTHOG_HOST=${PUBLIC_POSTHOG_HOST}
ENV PUBLIC_TYPESENSE_HOST=${PUBLIC_TYPESENSE_HOST}
ENV PUBLIC_TYPESENSE_API_KEY=${PUBLIC_TYPESENSE_API_KEY}

WORKDIR /app/website

# Build the website (static export to 'dist' directory)
RUN pnpm run build

# =============================================================================
# Stage 2: Serve with Caddy
# =============================================================================
FROM caddy:alpine

# Copy Caddyfile configuration
COPY website/Caddyfile /etc/caddy/Caddyfile

# Copy built files from builder stage
COPY --from=builder /app/website/dist /srv

# Default port (platform injects PORT env var)
ENV PORT=80

# Caddy automatically reads PORT from environment
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
