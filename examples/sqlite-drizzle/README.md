# Drizzle Integration

Demonstrates Drizzle ORM integration with Rivet Actors for type-safe database operations.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sqlite-drizzle
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

- **Schema** ([`src/db/schema.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sqlite-drizzle/src/db/schema.ts)): Defines the `users_table` table with Drizzle
- **Actor Definition** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sqlite-drizzle/src/index.ts)): Defines the `userDirectory` actor, which stores and lists users in its own SQLite database through Drizzle

## Resources

Read more about [SQLite](https://rivet.dev/actors/docs/sqlite), [Drizzle](https://rivet.dev/actors/docs/sqlite-drizzle), and [actions](https://rivet.dev/actors/docs/actions).

## License

MIT
