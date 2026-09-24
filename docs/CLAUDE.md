# Docs Bundle CLAUDE.md

Rules for the docs in this repo. These pages are **not** rendered here — they are
published on [rivet.dev](https://rivet.dev) by the
[rivet-website](https://github.com/rivet-dev/website) repo, which symlinks
this directory in. Everything below exists so a page written here renders
correctly there.

## Layout

Three bundles live here. Each is synced to the website independently by its own
workflow in `.github/workflows/docs-sync-*.yml`.

```
docs/
  actors/                 -> /actors/docs/... and /guides/...
    sidebar.json
    content/
      docs/**.mdx         -> /actors/docs/...
      learn/**.mdx        -> /guides/...          (re-rooted by the website)
  general/                -> /docs/...
    sidebar.json
    content/**.mdx        -> /docs/...
  integrations/           -> /integrations/...
    sidebar.json
    content/
      docs/**.mdx         -> /integrations/...
```

The website links each bundle's `content` into its content collection, so **only
real pages belong under `content/`**. Anything else (scripts, fixtures, notes)
goes elsewhere in the repo or it will be published as a docs page.

### Which bundle does a page belong in?

Ask whether the page would change if you swapped out the actor programming
model. If no, it is `general`: it describes the control plane that schedules,
routes, versions, and observes workloads. If yes, it is `actors`: it describes
the RivetKit SDK surface you write code against.

`general` is flat, with no subdirectories, because `/docs/` has no tab
dimension the way a product vertical does. `content/cli.mdx` renders at
`/docs/cli`.

`integrations` covers third-party frameworks and SDKs. Its `sidebar.json` is
intentionally empty: the website builds that navigation from
`src/data/integrations.ts` so it can carry vendor logos and category groups.

## Frontmatter

Every page needs `title` and `description`. Both are used for SEO and the
sidebar falls back to `title` when a sidebar entry omits one.

```mdx
---
title: "In-Memory State"
description: "Actors store state in memory for instant reads and writes."
---
```

## sidebar.json

Navigation for the bundle's tabs. Icons travel as Font Awesome **export names**,
not objects, so this repo needs no dependency on the website's icon package.

```json
{
  "docs": [
    { "title": "General", "pages": [
      { "title": "Introduction", "href": "/actors/docs", "icon": "faSquareInfo" }
    ]}
  ],
  "learn": []
}
```

- One key per content directory. `docs/actors` uses `docs` and `learn`; the other
  two bundles use `docs` alone.
- `href` is the full site path the page renders at, so it differs per bundle:
  `/actors/docs/...`, `/docs/...`, `/integrations/...`. The `learn` section is the
  exception: author it as `/actors/learn/...` and the website re-roots it onto
  `/guides/...`.
- Adding a page to `content/` does not add it to the nav. Add it here too.
- The Deploy and Self-Host sections are **not** in these files. They are
  website-owned and generated there.

## Code

- **Never inline a fenced TypeScript block.** Real examples live in `examples/`
  and are embedded with `<CodeSnippet>`, so they are type-checked and cannot rot.
  A snippet that fails to compile fails the website build.
- Snippet paths are relative to **this repo's root**, so the same path works both
  here and on rivet.dev:
  ```mdx
  <CodeSnippet file="examples/docs/actors-state/durable-basic.ts" />
  ```
- Embed part of a file with `region="name"`, delimited in the source by
  `// docs:start name` / `// docs:end name`.
- Shell commands, YAML, Dockerfiles, and terminal output **may** be inline fenced
  blocks. The no-inline rule exists for type checking, which only applies to
  TypeScript.
- Every TypeScript snippet must include its imports and define everything it
  references. Use `@nocheck` only for API that does not exist on this branch yet.
- Use `<CodeGroup workspace>` for examples spanning multiple files, with each
  file as its own `<CodeSnippet>`.

## What does not belong here

- **Marketing pages.** They live in the website repo.
- **Deploy and self-hosting guides.** They are written once in the website repo
  and templated across every product. Do not write a per-product copy.
- **Website components.** Do not import from the website by relative path or
  alias; a page must render from the components the site already provides.

## Terminology

Applies to everything published on the website.

- The service that routes, schedules, and persists is the **control plane**.
  Never "engine", "server", or "orchestrator".
- A process running user code with the Rivet SDK is a **worker**. Never "envoy",
  "runner", "node", "compute", or "data plane".
- **Never use "agent" as a deployment noun.** Rivet ships agentOS and Actors is
  "where agents live"; the collision is unrecoverable.
- **"envoy" never appears in docs.** Envoy Proxy is a top-tier CNCF project.
  Internal code keeps its own names.
- **"Rivet Compute" is retired.** Where prose must name the managed offering it
  is **Rivet Cloud**, and it links to <https://dashboard.rivet.dev>.
- Spell the product `agentOS`, never `AgentOS`. Capitalize **Rivet Actor** as a
  proper noun, lowercase generic "actor".
- Always `rivet.dev`, never `rivet.gg`.

## Writing

- Write comments and prose as complete sentences. **Never use em dashes**; use
  periods instead.
- Do not document deltas. A reader who never saw the old version gains nothing
  from "this was renamed".

## Previewing locally

Clone the website next to this repo and run it. It detects the sibling
automatically and serves this directory's pages live:

```sh
git clone https://github.com/rivet-dev/website
cd rivet-website && pnpm install && pnpm dev
```

The website resolves this repo as the sibling `../rivet`, and reads all three
bundles from it.

`pnpm assemble` prints which checkout each product resolved to. To point at a
different checkout, repoint the symlink; it is gitignored and assemble leaves an
existing one alone:

```sh
ln -sfn /path/to/this/repo/docs/actors/content       src/content/docs/actors
ln -sfn /path/to/this/repo/docs/general/content      src/content/docs/docs
ln -sfn /path/to/this/repo/docs/integrations/content src/content/docs/integrations
```
