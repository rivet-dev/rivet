# Geo-Distributed Database

Store user session data in edge-local Rivet Actors so preferences and activity stay close to users.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/geo-distributed-database
npm install
npm run dev
```


## Features

- Creates a region-specific Rivet Actor using `createInRegion` and saves its region input in `onCreate`.
- Persists session preferences, recent activity, and last login time in the actor's SQLite database.
- Measures action latency to highlight the benefit of edge-local updates.
- Visualizes data locality with a world map and region indicators.

## Implementation

The UserSession actor saves its region input in `onCreate` and reads and updates session data directly in SQLite, so each update is a single transaction.
See the implementation in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/src/actors.ts).

The frontend connects with `createInRegion` and displays session locality from the actor's session data.
See the client UI in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/frontend/App.tsx) and the server wiring in [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/geo-distributed-database/src/server.ts).

## Resources

Read more about [state](/docs/actors/state), [SQLite](/docs/actors/sqlite), [actions](/docs/actors/actions), and [actor inputs](/docs/actors/input).

## License

MIT
