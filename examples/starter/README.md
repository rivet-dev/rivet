# Rivet Platform for RivetKit

Example project demonstrating Rivet cloud platform deployment with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js
- Rivet CLI (`npm install -g @rivet-dev/cli`)
- Rivet Cloud account

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/rivet
npm install
```

### Configuration

Set up your environment variables:

```sh
export RIVET_ENDPOINT=https://api.rivet.dev
export RIVET_SERVICE_TOKEN=your_service_token
export RIVET_PROJECT=your_project_id
export RIVET_ENVIRONMENT=your_environment
```

### Development

```sh
npm run dev
```

This will start the RivetKit server locally at http://localhost:6420.

### Testing the Client

In a separate terminal, run the client script to interact with your actors:

```sh
npm run client
```

### Deployment

Deploy to Rivet Cloud:

```sh
rivet deploy
```

Your Rivet Actors will be deployed as Rivet actors with automatic scaling and management.

## License

Apache 2.0
