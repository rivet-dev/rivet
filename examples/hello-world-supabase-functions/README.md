# Hello World - Supabase Functions

A minimal Rivet Actor counter running on Supabase Edge Functions with the WebAssembly runtime.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-supabase-functions
npm install
npm run dev
```

`rivet dev` runs a local Rivet engine and spawns `supabase functions serve` for you.

## Prerequisites

- [Supabase CLI](https://supabase.com/docs/guides/cli)
- Docker, for Supabase's local Edge Runtime

## Implementation

The function calls `serve` from `@rivetkit/supabase`, which loads the WebAssembly runtime and serves the Rivet handler. `RIVET_ENDPOINT` is the only required variable.

See [`supabase/functions/rivet/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-supabase-functions/supabase/functions/rivet/index.ts).

## Resources

Read more about [actions](/docs/actors/actions) and [state](/docs/actors/state), or follow the [Supabase Functions Quickstart](/docs/actors/quickstart/supabase) and [deploy guide](/docs/deploy/supabase).

## License

MIT
