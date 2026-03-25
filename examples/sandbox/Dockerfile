FROM node:22-slim AS build
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
RUN pnpm build --filter=sandbox

FROM node:22-slim
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY examples/sandbox/package.json ./examples/sandbox/package.json
COPY --from=build /app/rivetkit-typescript/packages/ ./rivetkit-typescript/packages/
COPY --from=build /app/engine/sdks/typescript/ ./engine/sdks/typescript/
COPY --from=build /app/shared/typescript/ ./shared/typescript/

RUN pnpm install --prod --frozen-lockfile --filter=sandbox...

COPY --from=build /app/examples/sandbox/dist ./examples/sandbox/dist
COPY --from=build /app/examples/sandbox/public ./examples/sandbox/dist/public

WORKDIR /app/examples/sandbox

EXPOSE 8080
CMD ["pnpm", "start"]
