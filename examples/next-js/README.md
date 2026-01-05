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

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/next-js/src/backend/registry.ts)): Simple counter actor integrated with Next.js App Router

## Deployment

This example can be deployed to Vercel. See the [Vercel deployment guide](/docs/connect/vercel) for detailed instructions.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/rivet-dev/rivet/tree/main/examples/next-js&project-name=rivetkit-next-js&demo-title=RivetKit+Next.js&demo-description=Minimal+Next.js+example+with+Rivet+Actors)

### Required Environment Variables

After deploying, configure these environment variables in your Vercel project:
- `RIVET_ENDPOINT` - Your Rivet Engine endpoint
- `RIVET_NAMESPACE` - Your Rivet namespace
- `RIVET_RUNNER_TOKEN` - Your Rivet runner token

Get these values from the [Rivet dashboard](https://dashboard.rivet.dev) under Connect > Vercel.

## Resources

Read more about [Next.js integration](/docs/platforms/next-js), [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
