# @rivet-dev/flue

The Flue deployment target and Rivet runtime adapter. It plugs Rivet Actors
into Flue's runtime, exposing agents and workflows as deployable Rivet actors.

## Flue fork and Labs packages

This package depends on one compatibility set from the Rivet-maintained Flue
fork:

- Fork: `https://github.com/rivet-dev/flue`
- Upstream: `https://github.com/withastro/flue`
- npm packages:
  - `@rivet-dev/labs-flue-cli`
  - `@rivet-dev/labs-flue-runtime`
  - `@rivet-dev/labs-flue-sdk`

The fork contains the generic target-authoring and runtime adapter APIs proposed
upstream. `@rivet-dev/flue` remains in this repository and contains only the
Rivet-specific target, actors, and storage implementation.

### Package aliases are part of the contract

Flue source continues to import `@flue/cli`, `@flue/runtime`, and `@flue/sdk`.
Published packages preserve those names with npm aliases:

```json
{
  "@flue/cli": "npm:@rivet-dev/labs-flue-cli@<version>",
  "@flue/runtime": "npm:@rivet-dev/labs-flue-runtime@<version>",
  "@flue/sdk": "npm:@rivet-dev/labs-flue-sdk@<version>"
}
```

Inside the Flue workspace, renamed internal dependencies must use
`workspace:@rivet-dev/labs-flue-<package>@*`. pnpm rewrites that form to the
correct `npm:` alias while packing. A plain `workspace:*` can silently resolve
the upstream package instead.

CLI, runtime, and SDK are one compatibility set. Give all three the exact same
version, publish all three together, and never update Rivet or agentOS until all
three registry manifests have been inspected.

### Updating the fork and publishing a Labs set

Use a dedicated jj workspace for `rivet-dev/flue`. The remotes are:

- `github`: `withastro/flue`
- `fork`: `rivet-dev/flue`

Before changing the extension API:

```sh
jj git fetch --remote github
jj git fetch --remote fork
jj log -r 'main@github | main@fork'
```

Keep the fork based on current upstream `main` and preserve Flue's native router.
Use a version derived from upstream plus a Rivet revision, such as
`1.0.0-beta.9-rivet.2`.

1. Set the same version in:
   - `packages/sdk/package.json`
   - `packages/runtime/package.json`
   - `packages/cli/package.json`
2. Confirm every renamed workspace dependency uses
   `workspace:@rivet-dev/labs-flue-<package>@*`.
3. Confirm `scripts/prepare-publish.mjs` includes all three Labs packages.
4. Install and run focused gates:

   ```sh
   pnpm install
   pnpm --filter @rivet-dev/labs-flue-sdk build
   pnpm --filter @rivet-dev/labs-flue-sdk test
   pnpm --filter @rivet-dev/labs-flue-runtime build
   pnpm --filter @rivet-dev/labs-flue-runtime test
   pnpm --filter @rivet-dev/labs-flue-cli build
   pnpm --filter @rivet-dev/labs-flue-cli test
   node scripts/prepare-publish.mjs
   ```

5. Pack all three into a temporary directory and inspect each published
   `package.json`:

   ```sh
   pack_dir="$(mktemp -d)"
   pnpm --filter @rivet-dev/labs-flue-sdk pack --pack-destination "$pack_dir"
   pnpm --filter @rivet-dev/labs-flue-runtime pack --pack-destination "$pack_dir"
   pnpm --filter @rivet-dev/labs-flue-cli pack --pack-destination "$pack_dir"
   for archive in "$pack_dir"/*.tgz; do
     tar -xOf "$archive" package/package.json
   done
   ```

   The CLI manifest must alias both Labs SDK and Labs runtime at the same exact
   version.
6. Land the tested fork change. A push does not publish npm packages.
7. Publish SDK, runtime, then CLI:

   ```sh
   pnpm --filter @rivet-dev/labs-flue-sdk publish --access public --tag preview --no-git-checks
   pnpm --filter @rivet-dev/labs-flue-runtime publish --access public --tag preview --no-git-checks
   pnpm --filter @rivet-dev/labs-flue-cli publish --access public --tag preview --no-git-checks
   ```

8. Verify registry manifests before updating either consumer repository:

   ```sh
   npm view @rivet-dev/labs-flue-sdk@<version> version dependencies
   npm view @rivet-dev/labs-flue-runtime@<version> version dependencies
   npm view @rivet-dev/labs-flue-cli@<version> version dependencies
   ```

Never republish a version. If any manifest is wrong, increment the
`-rivet.N` suffix for the entire set.

### Updating every consumer to a new Labs version

Treat this as one cross-repository change. Search both repositories for the old
exact version and update every result that is source-controlled:

```sh
rg '<old-version>' \
  /path/to/rivet \
  /path/to/agentos \
  --glob '!**/node_modules/**' \
  --glob '!**/dist/**'
```

The required locations are:

- Rivet:
  - `integrations/flue-runtime/package.json`
  - `pnpm-lock.yaml`
  - `website/src/content/docs/integrations/flue.mdx`
- agentOS:
  - `examples/flue/package.json`
  - `packages/flue/package.json`
  - `pnpm-lock.yaml`
  - `website/src/content/docs/docs/frameworks/flue.mdx`

Generated `website/public/` or `website/dist/` copies are outputs, not sources;
regenerate them through the normal website build instead of editing them.

After updating aliases:

1. Run `pnpm install` in each repository.
2. Confirm the resolved CLI depends on the same Labs SDK and runtime version.
3. Build and test `@rivet-dev/flue`.
4. Typecheck/build the agentOS example and run the combined Flue E2E.
5. Pack `@rivet-dev/flue`, install all public artifacts in a fresh directory,
   and run the exact `npx flue run` command from the integration guide.

`@rivet-dev/flue` is published by Rivet's normal release process, not from the
Flue fork.

## Reference adapter: Cloudflare Durable Objects

This adapter is a **1:1 port of the Cloudflare Durable Object adapter** in the Flue runtime
(`@flue/runtime`, `packages/runtime/src/cloudflare/` in the flue monorepo). Both adapters implement
the same durable agent/workflow model (per-instance storage, the submission recovery model, the
run-record + central run-index topology) against a single-instance, per-actor SQLite backend.

**When in doubt about correct behavior, mimic the Cloudflare Durable Objects adapter.** It is the
reference implementation and the source of truth for the intended contract. Before changing or adding
behavior here, find the corresponding code in `src/cloudflare/` and match its semantics unless there is
a Rivet-specific reason to diverge — and when you must diverge, say so explicitly in a comment that
names the Cloudflare behavior you are departing from and why (e.g. Rivet has no durable resumable
fibers, so it omits the attempt-marker reconcile that Cloudflare needs).

Canonical examples of matching Cloudflare:
- **Run index is best-effort.** `createRun`/`endRun` write the authoritative per-actor record first,
  then mirror a pointer into the central registry actor. An index-write fault must be swallowed + logged,
  never propagated — it must not fail run admission or finalization. This matches Cloudflare's
  `safeIndexWrite` (`src/cloudflare/run-store.ts`). See `src/run-store-composite.ts`.
- **Recovery model.** Interrupted `running` submissions are reclaimed by reconcile + `claimSubmission`
  CAS, exactly as in `src/cloudflare/agent-coordinator.ts`.

Deliberate, documented divergences from Cloudflare:
- **No attempt markers in reconcile.** Cloudflare uses persisted attempt markers to avoid terminalizing
  a fiber that its SDK is auto-resuming after eviction. Rivet has no fiber resume (turns run via plain
  `keepAwake`), so every post-restart `running` submission is genuinely interrupted and should be
  reconciled. Wiring markers in would wrongly suppress legitimate recovery. (See `agent-coordinator.ts`.)
- **Transactions use the Rivet database transaction API.** Rivet wraps actor work in its own
  lifecycle, so adapter stores must not issue raw `BEGIN` statements. The `SAVEPOINT` fallback in
  `rivet-db.ts` exists only for the Node SQLite contract harness.
- **Response forwarding** crosses the actor RPC boundary (Cloudflare uses native DO RPC), so request/
  response (de)serialization lives here with no Cloudflare equivalent.

## Conventions

- TypeScript for all source/tests/fixtures/config; do not add `.mjs` files.
- Tests live in `test/`; design them from observable contracts, name them `it('X when Y')`.
- The store contract tests (shared with the runtime) must stay green; run them against the real
  `createRivetAsyncSqlDb` adapter, not a stronger double.
