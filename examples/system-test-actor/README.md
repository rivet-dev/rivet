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
BUILD=ws-isolate rivet shell -e "yarn test"
# or
BUILD=ws-container rivet shell -e "yarn test"
```

