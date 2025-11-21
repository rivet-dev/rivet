# Counter for RivetKit (Next.js)

Example Next.js project demonstrating basic actor state management and real-time updates with [RivetKit](https://rivetkit.org).

This example combines the counter functionality from the basic counter example with a Next.js application structure.

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/counter-next-js
pnpm install
```

### Development

```sh
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the counter in action.

The counter is shared across all clients using the same Counter ID. Try opening the page in multiple tabs or browsers to see real-time synchronization!

### Testing with the Connect Script

Run the connect script to interact with the counter from the command line:

```sh
pnpm connect
```

This will connect to the counter and increment it every second. You'll see the updates in both the terminal and the web interface!

## Features

- Real-time counter synchronization across multiple clients
- Next.js 15 with App Router
- TypeScript support
- Customizable counter IDs for multiple independent counters

## License

Apache 2.0
