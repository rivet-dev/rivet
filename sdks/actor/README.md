# Actor SDK

## Developing

Actors are usually tested by running tests against the `examples/` folder.

## Building for npm

```sh
    npm run build
```

## `allow-slow-types`

We allow [slow types](https://jsr.io/docs/about-slow-types) in our packages because we use Zod heavily, which relies on type inference.

