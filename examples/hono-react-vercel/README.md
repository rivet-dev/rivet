> **Note:** This is the Vercel-optimized version of the [hono-react](../hono-react) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fhono-react-vercel&project-name=hono-react-vercel)

# Hono + React

Example demonstrating full-stack Hono backend with React frontend integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hono-react
npm install
npm run dev
```


## Features

- **Full-stack Hono**: Hono web framework for HTTP routing and serving React frontend
- **React integration**: Complete frontend-backend integration with type-safe APIs
- **Actor backend**: Rivet Actors handle business logic and state management
- **Single codebase**: Frontend and backend in one project with shared types

## Implementation

This example demonstrates full-stack development with Hono and React:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hono-react/src/backend/registry.ts)): Shows full-stack integration with Hono backend and React frontend

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [setup](/docs/setup).

## License

MIT
