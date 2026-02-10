> **Note:** This is the Vercel-optimized version of the [per-tenant-database](../per-tenant-database) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fper-tenant-database-vercel&project-name=per-tenant-database-vercel)

# Per-Tenant Database

Example project demonstrating per-company database isolation with Rivet Actor state.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/per-tenant-database
npm install
npm run dev
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
