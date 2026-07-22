# Website Astro Dockerfile
# Multi-stage build: Node.js for building, Caddy for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
# Use Debian-based Node image (required for Playwright/mermaid rendering)
FROM node:22-bookworm AS builder

# Install pnpm — pin to the repo's packageManager version (NOT @latest). pnpm 11
# makes ERR_PNPM_IGNORED_BUILDS fatal (esbuild/sharp/core-js build scripts), which
# breaks the theme install below; 10.13.1 treats it as a non-fatal warning.
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate

WORKDIR /app

# Clone + build the shared docs theme at a pinned sha. The theme's dist/ (the
# compiled remark/rehype/generate-routes pipeline that docsTheme() imports) is
# gitignored, so we build it in-image. The committed website dep is a
# `github:rivet-dev/docs-theme#<sha>` pin for local dev; we repoint it at this
# clone below. Rivet uses its OWN @rivet-gg/components + icons (copied in from
# frontend/packages), so the theme's vendored copies are overridden via
# pnpm.overrides and no Font Awesome Pro token / icon generation is needed here.
ARG DOCS_THEME_SHA=450c498555135098c6a927adfdf13458be9be22a
RUN git clone https://github.com/rivet-dev/docs-theme.git /theme \
 && git -C /theme checkout "$DOCS_THEME_SHA" \
 && pnpm -C /theme install \
 && pnpm -C /theme/packages/theme build

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml turbo.json tsconfig.base.json tsup.base.ts ./

# Copy all workspace packages that website depends on
COPY website/ website/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/shared-data/ frontend/packages/shared-data/
COPY examples/ examples/
COPY integrations/ integrations/
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

# Repoint the docs-theme dep at the in-image clone+build (the committed value is
# a github sha pin). pnpm.overrides keep @rivet-gg/components+icons resolving to
# rivet's workspace copies. Install is non-frozen because this rewrite changes
# the manifest away from the committed github pin.
RUN node -e "const fs=require('fs'),p=require('./website/package.json');p.dependencies['@rivet-dev/docs-theme']='file:/theme/packages/theme';fs.writeFileSync('./website/package.json',JSON.stringify(p,null,2))"

# Install dependencies (with pnpm store cache)
RUN --mount=type=cache,id=s/3e31e381-166e-4a85-983a-a49830a88b96-/pnpm/store,target=/pnpm/store \
    pnpm install --no-frozen-lockfile

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

# Build the website (static export to 'dist' directory). Docs code blocks are
# now real, separately type-checked example files embedded via <CodeSnippet>, so
# there is no in-Astro code-block type checking to skip.
RUN pnpm run build

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
