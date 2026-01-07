# Website Dockerfile
# Multi-stage build: Node.js for building, nginx for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM node:22-alpine AS builder

# Install git and git-lfs for fetching LFS files
RUN apk add --no-cache git git-lfs

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./

# Copy all workspace packages that website depends on
# Need full source for packages with postinstall scripts (like icons)
COPY website/ website/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/example-registry/ frontend/packages/example-registry/
COPY examples/ examples/

# Fetch LFS files if build platform doesn't support Git LFS natively
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

# Arguments required before installing dependencies
ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

# Install dependencies (with pnpm store cache)
RUN --mount=type=cache,id=s/3e31e381-166e-4a85-983a-a49830a88b96-/pnpm/store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# Build arguments for NEXT_PUBLIC_* environment variables
ARG NEXT_PUBLIC_SITE_URL="https://rivet.gg"
ARG NEXT_PUBLIC_POSTHOG_KEY=""
ARG NEXT_PUBLIC_POSTHOG_HOST=""
ARG NEXT_PUBLIC_TYPESENSE_HOST=""
ARG NEXT_PUBLIC_TYPESENSE_API_KEY=""

# Set environment variables for build
ENV NEXT_PUBLIC_SITE_URL=${NEXT_PUBLIC_SITE_URL}
ENV NEXT_PUBLIC_POSTHOG_KEY=${NEXT_PUBLIC_POSTHOG_KEY}
ENV NEXT_PUBLIC_POSTHOG_HOST=${NEXT_PUBLIC_POSTHOG_HOST}
ENV NEXT_PUBLIC_TYPESENSE_HOST=${NEXT_PUBLIC_TYPESENSE_HOST}
ENV NEXT_PUBLIC_TYPESENSE_API_KEY=${NEXT_PUBLIC_TYPESENSE_API_KEY}

WORKDIR /app/website

# Build the website (static export to 'out' directory, with Next.js cache)
RUN --mount=type=cache,id=s/3e31e381-166e-4a85-983a-a49830a88b96-/app/website/.next/cache,target=/app/website/.next/cache \
    pnpm run build

# =============================================================================
# Stage 2: Serve with Caddy
# =============================================================================
FROM caddy:alpine

# Copy Caddyfile configuration
COPY website/Caddyfile /etc/caddy/Caddyfile

# Copy built files from builder stage
COPY --from=builder /app/website/out /srv

# Default port (platform injects PORT env var)
ENV PORT=80

# Caddy automatically reads PORT from environment
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
