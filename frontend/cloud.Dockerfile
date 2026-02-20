# Frontend (Cloud) Dockerfile
FROM node:22-alpine AS builder

RUN apk add --no-cache git git-lfs coreutils

RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Copy workspace configuration files
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml tsconfig.base.json turbo.json tsup.base.ts ./

# Copy frontend package
COPY frontend/ frontend/

# Copy engine SDK dependencies
COPY engine/sdks/typescript/api-full/ engine/sdks/typescript/api-full/
COPY engine/sdks/typescript/runner/ engine/sdks/typescript/runner/
COPY engine/sdks/typescript/runner-protocol/ engine/sdks/typescript/runner-protocol/

# Copy rivetkit dependencies
COPY rivetkit-typescript/packages/rivetkit/ rivetkit-typescript/packages/rivetkit/
COPY rivetkit-typescript/packages/traces/ rivetkit-typescript/packages/traces/
COPY rivetkit-typescript/packages/workflow-engine/ rivetkit-typescript/packages/workflow-engine/
COPY rivetkit-typescript/packages/sqlite-vfs/ rivetkit-typescript/packages/sqlite-vfs/

# Copy shared libraries
COPY shared/typescript/virtual-websocket/ shared/typescript/virtual-websocket/

# Copy examples and public assets
COPY examples/ examples/
COPY frontend/public/examples/ frontend/public/examples/

# Copy generated API docs
COPY rivetkit-asyncapi/ rivetkit-asyncapi/
COPY rivetkit-openapi/ rivetkit-openapi/

# Fetch LFS files
COPY scripts/docker/fetch-lfs.sh /tmp/fetch-lfs.sh
RUN chmod +x /tmp/fetch-lfs.sh && /tmp/fetch-lfs.sh

ARG FONTAWESOME_PACKAGE_TOKEN=""
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}

RUN --mount=type=cache,id=s/47975eb7-74fd-4043-a505-62b995ff5718-pnpm-store,target=/pnpm/store \
    pnpm install --frozen-lockfile

ARG VITE_APP_API_URL="https://VITE_APP_API_URL.placeholder.rivet.dev"
ARG VITE_APP_CLOUD_API_URL="https://VITE_APP_CLOUD_API_URL.placeholder.rivet.dev"
ARG VITE_APP_ASSETS_URL="https://VITE_APP_ASSETS_URL.placeholder.rivet.dev"
ARG VITE_APP_CLERK_PUBLISHABLE_KEY="pk_placeholder_clerk_key"
ARG VITE_APP_SENTRY_DSN="https://VITE_APP_SENTRY_DSN.placeholder.rivet.dev/0"
ARG VITE_APP_SENTRY_PROJECT_ID="0"
ARG VITE_APP_POSTHOG_API_KEY=""
ARG VITE_APP_POSTHOG_HOST=""
ARG DEPLOYMENT_TYPE="staging"

ENV VITE_APP_API_URL=${VITE_APP_API_URL}
ENV VITE_APP_CLOUD_API_URL=${VITE_APP_CLOUD_API_URL}
ENV VITE_APP_ASSETS_URL=${VITE_APP_ASSETS_URL}
ENV VITE_APP_CLERK_PUBLISHABLE_KEY=${VITE_APP_CLERK_PUBLISHABLE_KEY}
ENV VITE_APP_SENTRY_DSN=${VITE_APP_SENTRY_DSN}
ENV VITE_APP_SENTRY_PROJECT_ID=${VITE_APP_SENTRY_PROJECT_ID}
ENV VITE_APP_POSTHOG_API_KEY=${VITE_APP_POSTHOG_API_KEY}
ENV VITE_APP_POSTHOG_HOST=${VITE_APP_POSTHOG_HOST}
ENV VITE_APP_SENTRY_ENV=${RAILWAY_ENVIRONMENT_NAME:-staging}
ENV DEPLOYMENT_TYPE=${DEPLOYMENT_TYPE}
ENV FONTAWESOME_PACKAGE_TOKEN=${FONTAWESOME_PACKAGE_TOKEN}
ENV VITE_APP_SENTRY_TUNNEL="/tunnel"

RUN --mount=type=cache,id=s/47975eb7-74fd-4043-a505-62b995ff5718-turbo,target=/app/.turbo \
    npx turbo run build:cloud --filter=@rivetkit/engine-frontend

FROM caddy:alpine

RUN apk add --no-cache bash

COPY frontend/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /app/frontend/dist /srv
COPY frontend/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

ENV PORT=80
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile", "--adapter", "caddyfile"]
