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
