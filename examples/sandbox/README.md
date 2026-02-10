# Sandbox

Unified sandbox showcasing Rivet Actor features with a single registry, grouped navigation, and interactive demos.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sandbox
npm install
npm run dev
```


## Features

- Unified registry that aggregates actor fixtures and example actors
- Sidebar navigation grouped by core actor feature areas
- Action runner and event listener for quick experimentation
- Raw HTTP and WebSocket demos for handler-based actors
- Workflow and queue pattern coverage in a single sandbox

## Prerequisites

- OpenAI API key (set `OPENAI_API_KEY`) for the AI actor demo

## Implementation

The sandbox registry imports fixtures and example actors into one setup so each page can expose a curated subset.

See the registry in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox/src/actors.ts) and the UI in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox/frontend/App.tsx).

## Resources

Read more about [Rivet Actors](https://rivet.dev/docs/actors),
[actions](https://rivet.dev/docs/actors/actions), and
[connections](https://rivet.dev/docs/actors/connections).

## License

MIT
