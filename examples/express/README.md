# Express Integration

Example project demonstrating Express web framework integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/express
npm install
npm run dev
```


## Features

- **Express web framework**: Use Express for HTTP routing and middleware with actors
- **Familiar API**: Standard Express patterns and middleware work seamlessly
- **Actor integration**: Call actor actions from Express route handlers
- **Type-safe communication**: TypeScript support for actor method calls

## Implementation

This example demonstrates integrating Express web framework with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/express/src/backend/registry.ts)): Shows how to use Express middleware and routing with actors

## Deployment

This example can be deployed to Railway. See the [Railway deployment guide](/docs/connect/railway) for detailed instructions.

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https://github.com/rivet-dev/rivet/tree/main/examples/express)

### Required Environment Variables

Configure these environment variables in your Railway project:
- `RIVET_ENDPOINT` - Your Rivet Engine endpoint
- `RIVET_NAMESPACE` - Your Rivet namespace
- `RIVET_RUNNER_TOKEN` - Your Rivet runner token

Get these values from the [Rivet dashboard](https://dashboard.rivet.dev) under Connect > Railway.

## Resources

Read more about [actions](/docs/actors/actions) and [setup](/docs/setup).

## License

MIT
