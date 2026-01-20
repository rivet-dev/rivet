> **Note:** This is the Vercel-optimized version of the [raw-websocket-handler](../raw-websocket-handler) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fraw-websocket-handler-vercel&project-name=raw-websocket-handler-vercel)

# Raw WebSocket Handler

Demonstrates raw WebSocket handling with direct actor connections and real-time chat functionality.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/raw-websocket-handler
npm install
npm run dev
```


## Features

- **Raw WebSocket handlers**: Use `onWebSocket` for low-level WebSocket control and custom protocols
- **Direct actor connections**: Connect WebSocket clients directly to actor instances
- **Connection management**: Track WebSocket connections with state and broadcasting
- **Real-time chat**: Message broadcasting with user presence and chat history
- **Persistent state**: Messages and user data automatically saved in actor state

## Implementation

This example demonstrates raw WebSocket handling for real-time chat:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/raw-websocket-handler/src/backend/registry.ts)): Uses `onWebSocket` handler for low-level WebSocket protocol control

## Resources

Read more about [WebSockets](/docs/actors/websockets), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
