FROM node:22-slim

WORKDIR /app

# Install pnpm (match workspace version) and set CI for non-interactive behavior
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
ENV CI=true

# Copy and build SDK
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml tsup.base.ts tsconfig.base.json turbo.json .
COPY sdks/typescript/runner-protocol ./sdks/typescript/runner-protocol/
COPY sdks/typescript/tunnel-protocol ./sdks/typescript/tunnel-protocol/
COPY sdks/typescript/runner ./sdks/typescript/runner/

# Install dependencies
COPY sdks/typescript/test-runner/package.json ./sdks/typescript/test-runner/
RUN pnpm install --no-frozen-lockfile

# Build the application
COPY sdks/typescript/test-runner/ ./sdks/typescript/test-runner/
RUN pnpm build -F @rivetkit/engine-test-runner

# Expose the HTTP server port
EXPOSE 5050

# Run the application
CMD ["node", "sdks/typescript/test-runner/dist/main.js"]