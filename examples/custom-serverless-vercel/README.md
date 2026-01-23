> **Note:** This is the Vercel-optimized version of the [custom-serverless](../custom-serverless) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fcustom-serverless-vercel&project-name=custom-serverless-vercel)

# Custom Serverless

Example demonstrating custom serverless actor deployment with automatic engine configuration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/custom-serverless
npm install
npm run dev
```


## Features

- **Custom serverless deployment**: Configure and deploy actors to your own serverless infrastructure
- **Automatic engine configuration**: RivetKit automatically manages actor engine settings
- **Minimal setup**: Simple starter template for building custom deployments
- **Type-safe actions**: Full TypeScript support for actor definitions

## Implementation

This example shows how to deploy Rivet Actors to custom serverless platforms:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/custom-serverless/src/backend/registry.ts)): Demonstrates actor configuration for custom serverless deployment

## Resources

Read more about [setup](/docs/setup), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT
