FROM node:22-bullseye

# Install corepack globally as root, then fix permissions for user 1000:1000
RUN npm install -g corepack && \
    corepack enable && \
    chown -R 1000:1000 /usr/local/lib/node_modules/corepack* /usr/local/bin/yarn /usr/local/bin/pnpm /usr/local/bin/pnpx || true

# Switch to user 1000:1000
USER 1000:1000

# Set working directory
WORKDIR /app