# Hello World - Netlify

A minimal example demonstrating RivetKit with a real-time counter shared across multiple clients, deployed on Netlify Functions.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-netlify
npm install
npm run dev
```


## Deployment

1. Push your code to a Git repository
2. Connect your repository to [Netlify](https://netlify.com)
3. Configure environment variables in Netlify dashboard:
   - `RIVET_ENDPOINT`
   - `RIVET_PUBLIC_ENDPOINT` 
   - `RIVET_TOKEN`
4. Deploy your site

## Features

- **Actor state management**: Persistent counter state managed by Rivet Actors
- **Real-time updates**: Counter values synchronized across all connected clients via events
- **Multiple actor instances**: Each counter ID creates a separate actor instance
- **React integration**: Uses `@rivetkit/react` for seamless React hooks integration
- **Netlify Functions**: Optimized for serverless deployment on Netlify

## Implementation

This example demonstrates the core RivetKit concepts with a simple counter:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-netlify/src/actors.ts)): Counter actor with persistent state and broadcast events
- **Server Setup** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-netlify/src/server.ts)): Hono server with RivetKit handler
- **Netlify Function** ([`functions/rivet.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-netlify/functions/rivet.ts)): Handler that converts Netlify events to RivetKit requests
- **React Frontend** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-netlify/frontend/App.tsx)): Counter component using `useActor` hook and event subscriptions

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
