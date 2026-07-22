# Flue Rivet Example

A minimal `target: rivet` app: a faux local model provider, one HTTP agent (`agents/assistant.ts`), and
one workflow (`workflows/dispatch.ts`) that dispatches to the agent.

```sh
pnpm install
pnpm build      # flue build  -> dist/server.mjs
pnpm dev        # flue dev
```

`pnpm dev` boots its own Rivet engine by default (`RIVET_RUN_ENGINE=1`). To point at an external engine
instead, set `RIVET_ENDPOINT`, `RIVET_POOL`, and `FLUE_RIVET_REGISTRY_KEY` — see the package README at
`../../README.md` for the full by-hand walkthrough (engine config, runner-config PUT, and example
requests).

## What it exercises

- `POST /agents/assistant/<id>?wait=result` — synchronous prompt; the faux provider replies
  `"Hello from Rivet."`.
- `POST /workflows/dispatch?wait=result` — returns a `dispatchId` and dispatches `{ message }` to agent
  `assistant` instance `dispatched-<id>`, whose turn replies `"Hello from workflow dispatch."`.
- `GET /agents/assistant/dispatched-<id>` — the dispatched turn's output lands in that instance's event
  stream (dispatch is asynchronous, so poll it).
- `GET /runs/<runId>` and `GET /admin/runs` — workflow run inspection.

The end-to-end version of this flow is asserted in `../../test/rivet-examples.test.ts`
(`pnpm test:e2e`).
