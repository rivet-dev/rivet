# System Test

Actors for simple tests of E2E system functionality.

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## Deploying

```sh
rivet login
rivet deploy
```

## Testing

```sh
rivet deno --populate-env run -A ws_test.ts
```

