# Chat Room (Next.js)

Next.js chat room demonstrating real-time messaging with actor state management.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/chat-room-next-js
npm install
npm run dev
```


## Features

- **Next.js integration**: Use RivetKit actors with Next.js App Router and server actions
- **Real-time messaging**: Broadcast messages to all connected clients instantly
- **Persistent chat history**: Message history automatically saved in actor state
- **Multiple chat rooms**: Each room is a separate actor instance with isolated state

## Implementation

This example demonstrates using Rivet Actors with Next.js:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-next-js/src/backend/registry.ts)): Defines the `chatRoom` actor integrated with Next.js App Router

## Resources

Read more about [Next.js integration](/docs/platforms/next-js), [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
