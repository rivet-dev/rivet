# ai-agent-shopper

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## File structure

- `rivet.json` Configuration file for deploying the actor
- `src/shopper_agent.ts` Agent source code, deployed to Rivet
- `src/catalog_items.ts` Full list of items available
- `src/cli.ts` CLI to interact with the agent

## Running

```sh
# Deploy
rivet deploy

# Run interactive chat interface (also supports NodeJS)
rivet deno run -A --unstable-sloppy-imports src/cli.ts
```
