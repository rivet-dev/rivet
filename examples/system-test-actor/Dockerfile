# Build stage
FROM node:22 as builder
WORKDIR /app
COPY package.json yarn.lock ./
RUN yarn install
COPY . .
RUN yarn build

# Production stage
FROM node:22-slim
WORKDIR /app

# Create rivet user and set proper permissions
RUN groupadd -r rivet && useradd -r -g rivet rivet
COPY package.json yarn.lock ./
RUN yarn install --production && \
    chown -R rivet:rivet /app

COPY --from=builder /app/dist ./dist
RUN chown -R rivet:rivet /app/dist

# Switch to non-root user
USER rivet

# Start the server
CMD ["node", "dist/src/container/main.js"]
