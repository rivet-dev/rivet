# Per-Tenant Database

Example project demonstrating per-company database isolation with Rivet Actor state.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/per-tenant-database
pnpm install
pnpm dev
```

## Features

- **Per-tenant isolation**: Each company name maps to its own CompanyDatabase actor and state
- **State as the database**: Employees and projects live in `c.state` for every actor instance
- **Action-driven updates**: Add and list data through actions like `addEmployee` and `listProjects`
- **Live switching**: Swap company names in the UI to see fully isolated datasets

## Implementation

The dashboard connects to one CompanyDatabase actor per company. The actor key is the company name, and its state becomes that company database.

See the implementation in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/per-tenant-database/src/actors.ts) and [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/per-tenant-database/frontend/App.tsx).

## Resources

Read more about [state](/docs/actors/state), [actions](/docs/actors/actions), and [events](/docs/actors/events).

## License

MIT
