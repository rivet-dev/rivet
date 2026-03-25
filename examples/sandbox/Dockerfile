FROM node:22-slim AS build
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm run build

# Production image.
FROM node:22-slim
RUN corepack enable && corepack prepare pnpm@10.13.1 --activate
WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --prod --frozen-lockfile
COPY --from=build /app/dist ./dist
COPY --from=build /app/src ./src
COPY --from=build /app/sql-loader.mjs ./sql-loader.mjs
COPY --from=build /app/tsconfig.json ./tsconfig.json

EXPOSE 8080
CMD ["pnpm", "start"]
