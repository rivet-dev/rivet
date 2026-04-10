FROM node:22-slim
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app
ENV NODE_OPTIONS=--max-old-space-size=7168

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml turbo.json tsconfig.base.json tsup.base.ts ./
COPY examples/sandbox/ ./examples/sandbox/
COPY rivetkit-typescript/packages/ ./rivetkit-typescript/packages/
COPY engine/sdks/typescript/ ./engine/sdks/typescript/
COPY shared/typescript/ ./shared/typescript/

RUN pnpm install --frozen-lockfile
RUN mkdir -p rivetkit-openapi rivetkit-asyncapi

WORKDIR /app/examples/sandbox

CMD ["pnpm", "start"]
