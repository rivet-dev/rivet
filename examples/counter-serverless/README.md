# Counter (Serverless) for RivetKit

Example project demonstrating serverless actor deployment with automatic engine configuration using [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js
- RIVET_TOKEN environment variable (for serverless configuration)

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/counter-serverless
npm install
```

### Development

Set your Rivet token and run the development server:

```sh
export RIVET_TOKEN=your-token-here
npm run dev
```

Run the connect script to interact with the counter:

```sh
tsx scripts/connect.ts
```

## License

Apache 2.0