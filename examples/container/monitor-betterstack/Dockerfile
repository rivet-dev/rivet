# syntax=docker/dockerfile:1
# check=skip=SecretsUsedInArgOrEnv

FROM node:22-bookworm-slim

# Set environment variables including BetterStack credentials
ENV CI=true
ENV COREPACK_ENABLE_STRICT=0
ENV VECTOR_LOG=ERROR
RUN corepack enable

# Install Vector
RUN apt-get update -y && \
	apt-get install -y curl && \
	bash -c "$(curl -L https://setup.vector.dev)" && \
	apt-get install -y vector

# Configure Vector with BetterStack
COPY ./vector.yaml /etc/vector/vector.yaml

# Create a non-root user and set up directories
RUN groupadd -r rivet && useradd -r -g rivet rivet && \
    mkdir -p /home/rivet/.cache/node/corepack && \
    chown -R rivet:rivet /home/rivet

WORKDIR /app
COPY --chown=rivet:rivet . .

# Switch to rivet user and install dependencies
USER rivet
RUN yarn install

# Vector read logs from stdin and send to BetterStack
CMD ["sh", "-c", "NODE_ENV=production node ./server.js 2>&1 | vector -vv"]

