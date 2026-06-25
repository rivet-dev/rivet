FROM node:22-slim AS builder

WORKDIR /app

ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
ENV SKIP_TYPECHECK_CODE_BLOCKS=1

RUN corepack enable

COPY . .

RUN pnpm install --frozen-lockfile
RUN pnpm --filter rivet-website run build
RUN pnpm --filter @rivetkit/mcp-hub run build

FROM node:22-slim AS runner

WORKDIR /app

COPY --from=builder /app/rivetkit-typescript/packages/mcp-hub/package.json ./package.json
RUN npm install --omit=dev

COPY --from=builder /app/rivetkit-typescript/packages/mcp-hub/dist ./dist
COPY --from=builder /app/website/dist/metadata/docs.json ./metadata/docs.json

ENV PORT=7332
ENV DOCS_METADATA_PATH=/app/metadata/docs.json
EXPOSE 7332

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
	CMD node -e "fetch('http://localhost:' + (process.env.PORT || 7332) + '/mcp', {method: 'POST'}).then(r => process.exit(r.status < 500 ? 0 : 1)).catch(() => process.exit(1))"

CMD ["node", "dist/cli.js"]
