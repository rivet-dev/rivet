# Order Fulfillment State Machine for Rivet

Example project demonstrating a basic order state machine with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/workflows
npm install
```

### Development

```sh
npm run dev
```

Once the registry starts, the terminal prints the manager endpoint and inspector URL. Connect to `orderWorkflow` with any order ID (for example `order-123`), provide creation input like `{ "customer": "Acme Corp" }`, then use `advance` to step through the fulfillment stages and `getNextStatus` to see which state comes next.

## License

Apache 2.0
