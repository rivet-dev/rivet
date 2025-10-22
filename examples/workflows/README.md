# Order Fulfillment State Machine for RivetKit

Example project demonstrating a basic order state machine with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/workflows
npm install
```

### Development

```sh
npm run dev
```

Once the registry starts, the terminal prints the manager endpoint and inspector URL. Connect to `orderWorkflow` with any order ID (for example `order-123`), provide creation input like `{ "customer": "Acme Corp" }`, then use `advance` to step through the fulfillment stages and `getNextStatus` to see which state comes next.

## License

Apache 2.0
