# Flue Rivet Vercel Example

The same agent + dispatch workflow as `../rivet`, fronted by Next.js catch-all route handlers:

- `/api/flue/[...all]` serves the Flue front door through `toFlueNextHandler`.
- `/api/rivet/[...all]` serves the Rivet gateway through `@rivetkit/next-js`.

```sh
pnpm install
pnpm dev             # runs `flue build` (-> dist/server.mjs, the Rivet actor app) then `next dev`
```

The routes export `maxDuration = 300` for Rivet's long-lived serverless runner request. Normal agent
and workflow admissions return `202` without waiting for execution; Rivet owns that background work
through `c.keepAwake(...)`. Attached requests such as `?wait=result` and abrupt runner restarts are
still bounded by the serverless runtime, with Rivet `onWake` recovery handling interrupted work.

## Environment

`next dev` connects to a Rivet engine via the same variables as the package README
(`../../README.md`): `RIVET_POOL` and `FLUE_RIVET_REGISTRY_KEY` are required, plus either
`RIVET_RUN_ENGINE=1` (with `RIVET_RUN_ENGINE_HOST`/`_PORT`) to boot an engine, or `FLUE_RIVET_ENDPOINT`
pointing at the in-process Rivet gateway (`<site>/api/rivet`). `NEXT_PUBLIC_SITE_URL` should be the
app's own origin.

## Requests

Prompts go through the Flue route prefix:

```sh
curl -X POST "$ORIGIN/api/flue/agents/assistant/inst-1" \
  -H 'content-type: application/json' -d '{"kind":"user","body":"Hello"}'
# -> HTTP 202 with { "submissionId": "...", ... }

curl "$ORIGIN/api/flue/agents/assistant/inst-1?view=history"
# -> canonical conversation history, including "Hello from the Vercel route."
```

The build-and-serve flow is asserted end to end in `../../test/rivet-examples.test.ts`
(`pnpm test:e2e`).
