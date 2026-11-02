# Turso

Example project demonstrating a Turso Cloud database per Rivet Actor, created when the actor is created and deleted when it is destroyed.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/turso
npm install
cp .env.example .env # fill in your Turso organization and Platform API token
npm run dev
```

In another terminal, run the client:

```sh
npm run client
```

## Features

- **Database per actor**: `createState` creates a Turso database for each actor through the Platform API
- **Live objects in `c.vars`**: `createVars` opens a `@libsql/client` client to the actor's database on every start, and sleep drops it
- **Cleanup on destroy**: `onDestroy` deletes the actor's database

## Prerequisites

- A [Turso Cloud](https://turso.tech) organization and a Platform API token from `turso auth api-tokens mint`

## Implementation

The `todoList` actor creates its database and a scoped token in `createState`, then opens a client and creates its table in `createVars`. The client uses HTTP, so an idle client holds no connections and is never closed. `deleteList` destroys the actor, which runs `onDestroy`.

See the implementation in [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/turso/src/index.ts) and the test in [`tests/turso.test.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/turso/tests/turso.test.ts).

## Resources

Read the [Turso integration guide](https://rivet.dev/integrations/turso), and more about [state](https://rivet.dev/actors/docs/state), [lifecycle hooks](https://rivet.dev/actors/docs/lifecycle), and [destroying actors](https://rivet.dev/actors/docs/destroy).

## License

MIT
