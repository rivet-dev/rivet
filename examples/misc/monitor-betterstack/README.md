# BetterStack Monitoring

Ships container metrics & logs to BetterStack. Uses Vector to collect & ship logs under the hood.

## How it works

Vector will run as the parent process to your main process. It will collect logs & send all metrics to BetterStack automatically.

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)
- [Docker](https://docs.docker.com/desktop/)
- [Node.js](https://nodejs.org/) (for local development)

## Local Development

```sh
yarn install
PORT_HTTP=8080 yarn start
```

## Testing

### Using Docker

```sh
# Build with your BetterStack credentials
docker build -t bs-actor \
  --build-arg BETTERSTACK_TOKEN=your_token_here \
  --build-arg BETTERSTACK_HOST=logs.betterstack.com .

# Run the container
docker run bs-actor
```

### Using e2e_test.js

```sh
# Deploy to Rivet
rivet deploy

# Run the test with all required environment variables
BETTERSTACK_TOKEN=your_token_here \
BETTERSTACK_HOST=logs.betterstack.com \
RIVET_SERVICE_TOKEN=<TOKEN> \
RIVET_PROJECT=<PROJECT> \
RIVET_ENVIRONMENT=<ENV> \
yarn test
```

