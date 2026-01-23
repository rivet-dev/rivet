> **Note:** This is the Vercel-optimized version of the [cursors](../cursors) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fcursors-vercel&project-name=cursors-vercel)

# Real-time Collaborative Cursors

Example project demonstrating real-time cursor tracking and collaborative canvas.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cursors
npm install
npm run dev
```


## Features

- **Real-time cursor tracking**: Broadcast cursor positions to all connected users instantly
- **Event-driven architecture**: Use actor events to push updates to WebSocket clients
- **Persistent canvas state**: Text labels automatically saved in actor state across sessions
- **Multiple rooms**: Each room is a separate actor instance with isolated state

## Implementation

This example demonstrates real-time collaboration using WebSockets and Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cursors/src/backend/registry.ts)): Implements the `canvasRoom` actor for tracking cursor positions and managing collaborative canvas state

## Resources

Read more about [WebSockets](/docs/actors/websockets), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
