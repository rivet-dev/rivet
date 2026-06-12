# Website Astro Dockerfile
# Multi-stage build: Node.js for building, Caddy for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
# Use Debian-based Node image (required for Playwright/mermaid rendering)
FROM node:22-bookworm AS builder

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml turbo.json tsconfig.base.json tsup.base.ts ./

# Copy all workspace packages that website depends on
COPY website/ website/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/shared-data/ frontend/packages/shared-data/
COPY examples/ examples/
COPY engine/artifacts/ engine/artifacts/
COPY engine/sdks/typescript/ engine/sdks/typescript/
COPY rivetkit-typescript/artifacts/ rivetkit-typescript/artifacts/
COPY rivetkit-typescript/packages/ rivetkit-typescript/packages/
COPY rivetkit-openapi/ rivetkit-openapi/
COPY rivetkit-asyncapi/ rivetkit-asyncapi/
COPY shared/typescript/ shared/typescript/

# Arguments required before installing dependencies
ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

# Install dependencies (with pnpm store cache)
RUN --mount=type=cache,id=s/3e31e381-166e-4a85-983a-a49830a88b96-/pnpm/store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# Build arguments for PUBLIC_* environment variables
ARG PUBLIC_SITE_URL="https://rivet.dev"
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
RUN SKIP_TYPECHECK_CODE_BLOCKS=1 pnpm run build

# Generate the Caddy redirect snippet from the shared redirect map
RUN node scripts/generate-caddy-redirects.mjs

# =============================================================================
# Stage 2: Serve with Caddy
# =============================================================================
# Pin to caddy 2.x: the redirect snippet uses {http.request.uri.prefixed_query},
# which requires Caddy >= 2.8.
FROM caddy:2-alpine

# Copy Caddyfile configuration and the generated redirect snippet (imported by
# the Caddyfile relative to its own directory)
COPY website/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /app/website/redirects.caddy /etc/caddy/redirects.caddy

# Copy built files from builder stage
COPY --from=builder /app/website/dist /srv

# Fail the build on a broken config instead of crash-looping at runtime
RUN caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile

# Default port (platform injects PORT env var)
ENV PORT=80

# Caddy automatically reads PORT from environment
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
