> **Note:** This is the Vercel-optimized version of the [drizzle](../drizzle) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fdrizzle-vercel&project-name=drizzle-vercel)

# Drizzle Integration

Demonstrates Drizzle ORM integration with Rivet Actors for type-safe database operations.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/drizzle
npm install
npm run dev
```


## Features

- **Drizzle ORM integration**: Use Drizzle for type-safe database operations within actors
- **Automatic migrations**: Database schema migrations managed automatically
- **Type-safe queries**: Full TypeScript type safety from schema to queries
- **Actor-scoped database**: Each actor can have its own isolated database instance

## Implementation

This example demonstrates database integration with Rivet Actors using Drizzle ORM:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/drizzle/src/backend/registry.ts)): Shows how to integrate Drizzle ORM for type-safe database operations within actors

## Resources

Read more about [database integration](/docs/actors/database), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT
