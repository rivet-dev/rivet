> **Note:** This is the Vercel-optimized version of the [raw-websocket-handler-proxy](../raw-websocket-handler-proxy) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fraw-websocket-handler-proxy-vercel&project-name=raw-websocket-handler-proxy-vercel)

# Raw WebSocket Handler Proxy

Demonstrates raw WebSocket handling using a proxy endpoint pattern for routing connections to actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/raw-websocket-handler-proxy
npm install
npm run dev
```


## Features

- **Raw WebSocket handlers**: Use `onWebSocket` for low-level WebSocket control and custom protocols
- **Proxy endpoint pattern**: Route WebSocket connections through a proxy endpoint to actors
- **Connection management**: Track WebSocket connections with state and broadcasting
- **Real-time chat**: Message broadcasting with user presence and chat history
- **Persistent state**: Messages and user data automatically saved in actor state

## Implementation

This example demonstrates routing WebSocket connections through a proxy:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/raw-websocket-handler-proxy/src/backend/registry.ts)): Uses proxy endpoint pattern to route WebSocket connections to actors

## Resources

Read more about [WebSockets](/docs/actors/websockets), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
