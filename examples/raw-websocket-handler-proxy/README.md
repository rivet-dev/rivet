# Raw WebSocket Handler Proxy for RivetKit

Example project demonstrating raw WebSocket handling with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js 18 or later
- pnpm (for monorepo management)

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/raw-websocket-handler-proxy
npm install
```

### Development

```sh
npm run dev
```

This starts both the backend server (on port 9000) and the frontend development server (on port 5173).

Open http://localhost:5173 in your browser to see the chat application demo.

### Testing

```sh
npm test
```

## Features

This example demonstrates:

- Creating actors with raw WebSocket handlers using `onWebsocket`
- Managing WebSocket connections and broadcasting messages
- Maintaining actor state across connections
- Supporting multiple connection methods (direct actor connection vs proxy endpoint)
- Real-time chat functionality with user presence
- Message persistence and history limits
- User name changes
- Comprehensive test coverage

## License

Apache 2.0
