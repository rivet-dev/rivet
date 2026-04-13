FROM node:22-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# pnpm deploy produces a flat node_modules with no symlinks.
# dist/ contains the fully bundled vite output (ssr noExternal: true).
# dist/public/ contains the frontend static assets.
COPY . .

ENV PORT=8080
EXPOSE 8080
CMD ["node", "node_modules/srvx/bin/srvx.mjs", "dist/server.js"]
