# Hono + Bun Integration for RivetKit

Example project demonstrating Hono web framework with Bun runtime and React frontend integration with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Bun

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/hono-bun
npm install
```

### Development

```sh
npm run dev
```

This will start both the backend server (on port 8080) and the frontend dev server (on port 5173).

Open your browser to [http://localhost:5173](http://localhost:5173) to see the counter application.

You can also test the server directly by running:

```sh
curl -X POST http://localhost:6420/increment/test
```

## License

Apache 2.0
