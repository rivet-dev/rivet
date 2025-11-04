FROM node:22-slim

WORKDIR /app

# Install pnpm (match workspace version) and set CI for non-interactive behavior
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
ENV CI=true

# Copy and build SDK
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml tsup.base.ts tsconfig.base.json turbo.json .
COPY engine/sdks/typescript/runner-protocol ./engine/sdks/typescript/runner-protocol/
COPY engine/sdks/typescript/runner ./engine/sdks/typescript/runner/

# Install dependencies
COPY engine/sdks/typescript/test-runner/package.json ./engine/sdks/typescript/test-runner/
RUN pnpm install --no-frozen-lockfile

# Build the application
COPY engine/sdks/typescript/test-runner/ ./engine/sdks/typescript/test-runner/
RUN pnpm build -F @rivetkit/engine-test-runner

# Expose the HTTP server port
EXPOSE 5050

# Run the application
CMD ["node", "engine/sdks/typescript/test-runner/dist/index.js"]
