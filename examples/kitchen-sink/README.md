# Kitchen Sink Example

Example project demonstrating all RivetKit features.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/kitchen-sink
npm install
npm run dev
```


## Features

- **Complete feature showcase**: Demonstrates all major RivetKit concepts in one example
- **Lifecycle hooks**: Use `onWake`, `onSleep`, `onConnect`, and `onDisconnect` for actor lifecycle management
- **Scheduling**: Schedule delayed actions with `schedule.at()` and `schedule.after()`
- **HTTP and WebSocket**: Handle both HTTP requests and WebSocket connections in actors
- **Event broadcasting**: Broadcast events to all connected clients with `c.broadcast()`
- **Connection management**: Track and manage multiple client connections per actor

## Implementation

This comprehensive example brings together all Rivet Actor features in one place:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/kitchen-sink/src/backend/registry.ts)): Demonstrates the complete feature set including lifecycle hooks, scheduling, events, state management, and WebSocket handling

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), [lifecycle hooks](/docs/actors/lifecycle), [scheduling](/docs/actors/scheduling), [events](/docs/actors/events), and [WebSockets](/docs/actors/websockets).

## License

MIT
