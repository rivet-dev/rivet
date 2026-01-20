> **Note:** This is the Vercel-optimized version of the [cursors-raw-websocket](../cursors-raw-websocket) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fcursors-raw-websocket-vercel&project-name=cursors-raw-websocket-vercel)

# Real-time Collaborative Cursors (Raw WebSocket)

Demonstrates real-time cursor tracking and collaborative canvas using raw WebSocket handlers instead of RivetKit's higher-level WebSocket abstraction.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cursors-raw-websocket
npm install
npm run dev
```


## Features

- **Raw WebSocket handlers**: Use `onWebsocket` for low-level WebSocket control and custom protocols
- **Real-time cursor tracking**: Broadcast cursor positions to all connected users instantly
- **Persistent canvas state**: Text labels automatically saved in actor state across sessions
- **Multiple rooms**: Each room is a separate actor instance with isolated state

## Implementation

This example demonstrates low-level WebSocket handling for real-time collaboration:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cursors-raw-websocket/src/backend/registry.ts)): Uses raw `onWebsocket` handler for custom WebSocket protocol implementation

## Resources

Read more about [WebSockets](/docs/actors/websockets), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
