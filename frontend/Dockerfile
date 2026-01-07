# Frontend (Cloud) Dockerfile
# Multi-stage build: Node.js for building, nginx for serving

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM node:22-alpine AS builder

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./

# Copy all workspace packages that frontend depends on
# Need full source for packages with postinstall scripts (like icons)
COPY frontend/ frontend/
COPY engine/sdks/typescript/api-full/ engine/sdks/typescript/api-full/
COPY rivetkit-typescript/packages/rivetkit/ rivetkit-typescript/packages/rivetkit/

# Install dependencies
RUN pnpm install --frozen-lockfile

# Build arguments for environment variables
# Use placeholder URLs that pass validation but can be replaced at runtime
# Format: https://__PLACEHOLDER__.rivet.gg allows easy sed replacement
ARG VITE_APP_API_URL="https://VITE_APP_API_URL.placeholder.rivet.gg"
ARG VITE_APP_CLOUD_API_URL="https://VITE_APP_CLOUD_API_URL.placeholder.rivet.gg"
ARG VITE_APP_ASSETS_URL="https://VITE_APP_ASSETS_URL.placeholder.rivet.gg"
ARG VITE_APP_CLERK_PUBLISHABLE_KEY="pk_placeholder_clerk_key"
ARG VITE_APP_SENTRY_DSN="https://VITE_APP_SENTRY_DSN.placeholder.rivet.gg/0"
ARG VITE_APP_SENTRY_PROJECT_ID="0"
ARG VITE_APP_POSTHOG_API_KEY=""
ARG VITE_APP_POSTHOG_HOST=""
ARG DEPLOYMENT_TYPE="staging"

# Set environment variables for build
ENV VITE_APP_API_URL=${VITE_APP_API_URL}
ENV VITE_APP_CLOUD_API_URL=${VITE_APP_CLOUD_API_URL}
ENV VITE_APP_ASSETS_URL=${VITE_APP_ASSETS_URL}
ENV VITE_APP_CLERK_PUBLISHABLE_KEY=${VITE_APP_CLERK_PUBLISHABLE_KEY}
ENV VITE_APP_SENTRY_DSN=${VITE_APP_SENTRY_DSN}
ENV VITE_APP_SENTRY_PROJECT_ID=${VITE_APP_SENTRY_PROJECT_ID}
ENV VITE_APP_POSTHOG_API_KEY=${VITE_APP_POSTHOG_API_KEY}
ENV VITE_APP_POSTHOG_HOST=${VITE_APP_POSTHOG_HOST}
ENV DEPLOYMENT_TYPE=${DEPLOYMENT_TYPE}

WORKDIR /app/frontend

# Build the cloud frontend
RUN pnpm run build:cloud

# =============================================================================
# Stage 2: Serve with nginx
# =============================================================================
FROM nginx:alpine

# Install envsubst for runtime environment variable substitution
RUN apk add --no-cache bash

# Copy nginx configuration
COPY frontend/nginx.conf /etc/nginx/nginx.conf

# Copy built files from builder stage
COPY --from=builder /app/frontend/dist /usr/share/nginx/html

# Copy entrypoint script for runtime env var substitution
COPY frontend/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

# Expose port 80
EXPOSE 80

# Use custom entrypoint for env var substitution
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
