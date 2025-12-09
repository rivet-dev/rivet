# Raw Fetch Handler Example

Example project demonstrating raw HTTP fetch handling with Hono integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/raw-fetch-handler
npm install
npm run dev
```


## Features

- **Raw fetch handlers**: Use `onRequest` for low-level HTTP request handling with custom routing
- **Hono integration**: Embed Hono router inside actor fetch handlers using `createVars`
- **HTTP endpoints**: Define custom HTTP endpoints directly within actors
- **Proxy routing**: Forward HTTP requests from external endpoints to actor fetch handlers
- **Multiple actor instances**: Each named counter maintains independent state

## Implementation

The backend defines a counter actor with a Hono router embedded in the `onRequest` handler. Each counter is identified by a unique name, and the frontend can interact with counters through direct actor fetch calls or HTTP requests through a forward endpoint. Multiple counters maintain independent state.

### Key Implementation

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/raw-fetch-handler/src/backend/registry.ts)): Demonstrates `onRequest` handler with Hono router for custom HTTP routing

## Project Structure

```
raw-fetch-handler/
├── src/
│   ├── backend/     # RivetKit server with counter actors
│   └── frontend/    # React app demonstrating client interactions
└── tests/           # Vitest test suite
```

## Resources

Read more about [HTTP request handling](/docs/actors/http), [state](/docs/actors/state), and [actions](/docs/actors/actions).

## License

MIT
