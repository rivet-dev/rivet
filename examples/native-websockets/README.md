# Native WebSockets

Demonstrates native WebSocket integration with Rivet Actors for real-time bidirectional communication.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/native-websockets
npm install
npm run dev
```


## Features

- **Native WebSocket support**: Use standard WebSocket APIs for real-time communication
- **Bidirectional messaging**: Send and receive messages between client and actor
- **Connection management**: Track WebSocket connections with `onConnect` and `onDisconnect` hooks
- **Event broadcasting**: Push updates to all connected WebSocket clients

## Implementation

This example demonstrates native WebSocket integration with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/native-websockets/src/backend/registry.ts)): Shows how to use native WebSocket APIs with actors for real-time communication

## Resources

Read more about [WebSockets](/docs/actors/websockets), [events](/docs/actors/events), and [lifecycle hooks](/docs/actors/lifecycle).

## License

MIT
