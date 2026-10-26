# Per-Tenant Database

Example project demonstrating per-company database isolation with one Rivet Actor and one SQLite database per tenant.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/per-tenant-database
npm install
npm run dev
```


## Features

- **Per-tenant isolation**: Each company name maps to its own CompanyDatabase actor and SQLite database
- **SQL tables per tenant**: Employees and projects live in tables created by the actor's `db({ onMigrate })` migration
- **Action-driven updates**: Add and list data through actions like `addEmployee` and `listProjects`, with `employeeAdded` and `projectAdded` events pushed to every connected client
- **Live switching**: Swap company names in the UI to see fully isolated datasets

## Implementation

The dashboard connects to one CompanyDatabase actor per company. The actor key is the company name, and the actor's SQLite database is that company's database.

See the implementation in [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/per-tenant-database/src/index.ts) and [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/per-tenant-database/frontend/App.tsx).

## Resources

Read the [Database per Tenant guide](https://rivet.dev/guides/per-tenant-database), and more about [SQLite](https://rivet.dev/actors/docs/sqlite), [actions](https://rivet.dev/actors/docs/actions), and [events](https://rivet.dev/actors/docs/events).

## License

MIT
