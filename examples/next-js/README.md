# Next.js

Minimal Next.js example demonstrating basic actor state management and real-time updates.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/next-js
npm install
npm run dev
```


## Features

- **Next.js 15 integration**: Use RivetKit with Next.js App Router and server actions
- **Real-time updates**: Counter values synchronized across all connected clients
- **Actor state management**: Persistent counter state managed by Rivet Actors
- **Multiple actor instances**: Each counter ID creates a separate actor instance

## Implementation

This example demonstrates minimal Next.js integration with Rivet Actors:

- **Actor Definition** ([`src/rivet/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/next-js/src/rivet/actors.ts)): Simple counter actor integrated with Next.js App Router

## Resources

Read more about [Next.js integration](/docs/platforms/next-js), [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
