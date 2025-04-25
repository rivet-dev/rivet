# System Test

Actors for simple tests of E2E system functionality.

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)

## Deploying

```sh
rivet deploy
```

## Testing

Build the API:

```sh
cd sdks/api/full/typescript
yarn build
```

Run the test:

```sh
BUILD=http-isolate rivet shell -e "yarn test"
# or
BUILD=http-container rivet shell -e "yarn test"
```

