> **Note:** This is the Vercel-optimized version of the [chat-room](../chat-room) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fchat-room-vercel&project-name=chat-room-vercel)

# Chat Room

Example project demonstrating real-time messaging and actor state management.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/chat-room
npm install
npm run dev
```


## Features

- **Real-time messaging**: Broadcast messages to all connected clients instantly
- **Persistent chat history**: Messages automatically saved in actor state across restarts
- **Multiple chat rooms**: Each room is a separate actor instance with isolated state
- **Event-driven architecture**: Use actor events to push updates to clients in real-time

## Implementation

The chat room demonstrates core Rivet Actor patterns for real-time communication:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room/src/backend/registry.ts)): Defines the `chatRoom` actor with message history state and actions for sending messages and retrieving history

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
