# State Management

Demonstrates durable data in Rivet Actors: messages are stored in the actor's embedded SQLite database and read back with SQL.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/state
npm install
npm run dev
```


## Features

- **Durable data in SQLite**: Each actor writes its messages to its own embedded SQLite database, so they survive restarts and sleep
- **Direct reads**: `getMessages` queries SQLite on demand, so there is no in-memory copy to keep in sync
- **Schema migrations**: `onMigrate` creates the table before any other lifecycle hook runs

## Implementation

This example demonstrates durable data in Rivet Actors with a simple chat room:

- **Actor Definition** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/state/src/index.ts)): Defines the `chatRoom` actor, which stores and reads messages in SQLite

## Resources

Read more about [state](https://rivet.dev/actors/docs/state), [SQLite](https://rivet.dev/actors/docs/sqlite), [actions](https://rivet.dev/actors/docs/actions), and [lifecycle hooks](https://rivet.dev/actors/docs/lifecycle).

## License

MIT
