> **Note:** This is the Vercel-optimized version of the [hello-world](../hello-world) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fhello-world-vercel&project-name=hello-world-vercel)

# Hello World

A minimal example demonstrating RivetKit with a real-time counter shared across multiple clients.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world
npm install
npm run dev
```


## Features

- **Actor state management**: Persistent counter state managed by Rivet Actors
- **Real-time updates**: Counter values synchronized across all connected clients via events
- **Multiple actor instances**: Each counter ID creates a separate actor instance
- **React integration**: Uses `@rivetkit/react` for seamless React hooks integration

## Implementation

This example demonstrates the core RivetKit concepts with a simple counter:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world/src/actors.ts)): Counter actor with persistent state and broadcast events
- **Server Setup** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world/src/server.ts)): Minimal Hono server with RivetKit handler
- **React Frontend** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world/frontend/App.tsx)): Counter component using `useActor` hook and event subscriptions

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
