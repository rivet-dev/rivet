> **Note:** This is the Vercel-optimized version of the [geo-distributed-database](../geo-distributed-database) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fgeo-distributed-database-vercel&project-name=geo-distributed-database-vercel)

# Geo-Distributed Database

Store user session state in edge-local Rivet Actors so preferences and activity stay close to users.

## Getting Started

```bash
pnpm install
pnpm dev
```

## Features

- Creates a region-specific Rivet Actor using `createInRegion` and `createState` input.
- Persists session preferences, recent activity, and last login time in actor state.
- Measures action latency to highlight the benefit of edge-local updates.
- Visualizes data locality with a world map and region indicators.

## Implementation

The UserSession actor initializes state with a region input and stores session data in persistent state.
See the implementation in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/src/actors.ts).

The frontend connects with `createInRegion` and displays session locality from the actor state.
See the client UI in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/frontend/App.tsx) and the server wiring in [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/src/server.ts).

## Resources

Read more about [state](/docs/actors/state), [actions](/docs/actors/actions), and [actor inputs](/docs/actors/input).

## License

MIT
