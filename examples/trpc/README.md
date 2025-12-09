# tRPC Integration

Example project demonstrating tRPC integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/trpc
npm install
npm run dev
```


## Features

- **tRPC integration**: Use tRPC for type-safe API endpoints that call Rivet Actors
- **End-to-end type safety**: Full TypeScript types from frontend to actor actions
- **React Query integration**: Automatic caching and real-time updates with tRPC React hooks
- **Actor backend**: Rivet Actors handle business logic while tRPC provides the API layer

## Implementation

This example demonstrates integrating tRPC with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/trpc/src/backend/registry.ts)): Shows how to integrate tRPC router with actors for end-to-end type safety

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [setup](/docs/setup).

## License

MIT
