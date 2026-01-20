> **Note:** This is the Vercel-optimized version of the [multi-region](../multi-region) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fmulti-region-vercel&project-name=multi-region-vercel)

# Multi-Region

Demonstrates deploying Rivet Actors across multiple geographic regions for low-latency global access.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/multi-region
npm install
npm run dev
```


## Features

- **Multi-region deployment**: Deploy actors across multiple geographic regions automatically
- **Low-latency access**: Users connect to the nearest region for optimal performance
- **Automatic routing**: Requests automatically routed to the appropriate regional deployment
- **Global state management**: Actor state synchronized across regions as needed

## Implementation

This example demonstrates deploying actors across multiple regions:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/multi-region/src/backend/registry.ts)): Shows configuration for multi-region actor deployment

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [setup](/docs/setup).

## License

MIT
