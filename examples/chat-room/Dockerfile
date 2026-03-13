# Build stage
FROM node:22-alpine AS builder

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package files
COPY package.json ./

# Install all dependencies (including devDependencies needed for build)
RUN pnpm install --frozen-lockfile=false

# Copy source files
COPY . .

# Build the app (frontend + server bundle)
RUN pnpm run build

# Runtime stage
FROM node:22-alpine AS runtime

WORKDIR /app

# Install pnpm for running start script
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package.json for dependency installation
COPY package.json ./

# Install only production dependencies
RUN pnpm install --prod --frozen-lockfile=false

# Copy built artifacts from builder
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/public ./dist/public

EXPOSE 8080

ENV PORT=8080
ENV NODE_ENV=production

CMD ["node_modules/.bin/srvx", "dist/server.js"]

# Note: srvx resolves --static relative to CWD (/app), so public/ maps to /app/public/
