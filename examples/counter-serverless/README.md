# Counter (Serverless) for Rivet

Example project demonstrating serverless actor deployment with automatic engine configuration using [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js
- RIVET_TOKEN environment variable (for serverless configuration)

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/counter-serverless
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