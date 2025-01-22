# ai-agent-shopper

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## File structure

- `rivet.json` Configuration file for deploying the actor
- `shopper_agent.ts` Agent source code, deployed to Rivet
- `cli.ts` CLI to interact with the agent

## Running

```sh
rivet login
rivet deploy
rivet deno run -A cli.ts  # Also supports NodeJS
```
