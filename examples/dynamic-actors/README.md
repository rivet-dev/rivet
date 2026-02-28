# Dynamic Actors

Example showing a user-editable actor source workflow with `dynamicActor`.

## Getting Started

```sh
cd examples/dynamic-actors
pnpm install
pnpm dev
```

## Features

- Dynamic actor loading via `dynamicActor` from `rivetkit/dynamic`
- Actor-to-actor source loading where `dynamicWorkflow` loads code from `sourceCode`
- In-browser editor to update actor source at runtime
- User-controlled dynamic actor key input with one-click random key generation to force fresh actor loads

## Prerequisites

- Build `sandboxed-node` in your Secure Exec checkout and make it resolvable by this project
- If needed, set `RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER` to a file URL for `sandboxed-node/dist/index.js`

## Implementation

The actor definitions are in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/dynamic-actors/src/actors.ts).

- `sourceCode` stores editable source and revision in actor state
- `dynamicWorkflow` loads current source from `sourceCode` in its loader context, then evaluates and runs it

The server wiring is in [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/dynamic-actors/src/server.ts).

The UI is in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/dynamic-actors/frontend/App.tsx) and provides save + execute controls.

## Resources

Read more about [AI and user-generated Rivet Actors](/docs/actors/ai-and-user-generated-actors), [actions](/docs/actors/actions), and [communicating between actors](/docs/actors/communicating-between-actors).

## License

MIT
