# Website Dockerfile
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

# Copy all workspace packages that website depends on
# Need full source for packages with postinstall scripts (like icons)
COPY website/ website/
COPY frontend/packages/components/ frontend/packages/components/
COPY frontend/packages/icons/ frontend/packages/icons/
COPY frontend/packages/example-registry/ frontend/packages/example-registry/
COPY examples/ examples/

# Install dependencies
RUN pnpm install --frozen-lockfile

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

# Build the website (static export to 'out' directory)
RUN pnpm run build

# =============================================================================
# Stage 2: Serve with nginx
# =============================================================================
FROM nginx:alpine

# Copy nginx configuration
COPY website/nginx.conf /etc/nginx/nginx.conf

# Copy built files from builder stage
COPY --from=builder /app/website/out /usr/share/nginx/html

# Expose port 80
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
