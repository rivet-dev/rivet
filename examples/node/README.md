# Node.js

Minimal Node.js example demonstrating basic actor state management.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/node
npm install
npm run dev
```


## Features

- **Node.js runtime**: Run Rivet Actors on standard Node.js runtime
- **Type-safe actions**: Define and call actor actions with full TypeScript type safety
- **Persistent state**: Actor state automatically persisted and restored
- **Simple setup**: Minimal configuration to get started with RivetKit

## Implementation

This example demonstrates minimal Node.js setup with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/node/src/backend/registry.ts)): Basic actor setup for Node.js runtime

## Deployment

This example can be deployed to Railway. See the [Railway deployment guide](/docs/connect/railway) for detailed instructions.

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https://github.com/rivet-dev/rivet/tree/main/examples/node)

### Required Environment Variables

After deploying, configure these environment variables in your Railway project:
- `RIVET_ENDPOINT` - Your Rivet Engine endpoint
- `RIVET_NAMESPACE` - Your Rivet namespace
- `RIVET_RUNNER_TOKEN` - Your Rivet runner token

Get these values from the [Rivet dashboard](https://dashboard.rivet.dev) under Connect > Railway.

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [setup](/docs/setup).

## License

MIT
