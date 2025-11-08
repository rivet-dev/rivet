# Hono + Bun Integration for Rivet

Example project demonstrating Hono web framework with Bun runtime and React frontend integration with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Bun

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/hono-bun
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
curl -X POST http://localhost:8080/increment/test
```

## License

Apache 2.0
