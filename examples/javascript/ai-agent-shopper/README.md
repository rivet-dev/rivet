# ai-agent

A new project powered by [Rivet](https://rivet.gg).

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## File structure

- `rivet.json` Configuration file for deploying the actor
- `deno.json` Configuration file for dependencies for the actor
- `counter.ts` Simple counter actor
- `counter_test.ts` Script to test the counter

## Deploying

```sh
rivet login
rivet deploy
```

## Testing

```sh
rivet deno run -A counter_test.ts
```

