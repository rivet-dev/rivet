# System Test

Actors for simple tests of E2E system functionality.

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## Deploying

```sh
rivet deploy
```

## Testing

```sh
rivet shell -e "yarn test --isolate"
# or
rivet shell -e "yarn test --container"
```

