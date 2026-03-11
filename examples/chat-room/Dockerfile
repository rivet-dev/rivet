FROM node:22-alpine AS builder

WORKDIR /app

RUN corepack enable

COPY package.json ./
RUN pnpm install --no-frozen-lockfile

COPY src ./src
COPY frontend ./frontend
COPY index.html ./index.html
COPY tsconfig.json ./tsconfig.json
COPY turbo.json ./turbo.json
COPY vite.config.ts ./vite.config.ts
RUN pnpm build
RUN pnpm prune --prod

FROM node:22-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000

COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/public ./public
COPY --from=builder /app/dist ./dist

EXPOSE 3000

CMD ["pnpm", "start"]