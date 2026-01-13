# Base image for taking screenshots of examples
# This image includes Node.js, pnpm, Chromium, and the built rivetkit packages
#
# Build context should be the repository root:
#   docker build -t rivet-example-screenshot -f frontend/packages/example-registry/scripts/screenshotter/Dockerfile .
#
# IMPORTANT: Run `pnpm build` in the repo root first to build rivetkit packages,
# or they will be copied without dist folders and won't work.

FROM node:22-slim

# Install Chromium dependencies for Puppeteer
RUN apt-get update && apt-get install -y \
    chromium \
    fonts-liberation \
    libasound2 \
    libatk-bridge2.0-0 \
    libatk1.0-0 \
    libatspi2.0-0 \
    libcups2 \
    libdbus-1-3 \
    libdrm2 \
    libgbm1 \
    libgtk-3-0 \
    libnspr4 \
    libnss3 \
    libx11-xcb1 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libxkbcommon0 \
    libxrandr2 \
    xdg-utils \
    procps \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Tell Puppeteer to use system Chromium
ENV PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium

# Install pnpm and global tools
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
RUN npm install -g tsx puppeteer

WORKDIR /app

# Copy workspace config files (all root-level configs needed for builds)
COPY package.json pnpm-workspace.yaml ./
COPY turbo.json ./
COPY tsup.base.ts ./
COPY tsconfig.base.json ./
COPY vitest.base.ts ./
COPY biome.json ./

# Copy lockfile if it exists
COPY pnpm-lock.yaml* ./

# Copy rivetkit packages (with pre-built dist folders - run pnpm build first!)
COPY rivetkit-typescript ./rivetkit-typescript

# Copy engine SDK packages (needed by rivetkit)
COPY engine/sdks/typescript ./engine/sdks/typescript

# Copy shared packages (needed by engine SDK)
COPY shared ./shared

# Copy example-registry package (for puppeteer and screenshot script)
COPY frontend/packages/example-registry ./frontend/packages/example-registry

# Copy all examples
COPY examples ./examples

# Install all dependencies
RUN pnpm install --frozen-lockfile || pnpm install

WORKDIR /app

# Default command
CMD ["echo", "Use: docker run <image> tsx /app/frontend/packages/example-registry/scripts/screenshotter/take-screenshot.ts <example-name> <output-path>"]
