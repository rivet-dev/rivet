# Actor SDK

## Symlinking local dependencies

Deno does not allow importing JS files from outside of the package's directory. Therefore, we have to symlink the local package.

For example, we symlink `./core/src` to `./runtime/src/core` so the runtime package can access code from core.

## `allow-slow-types`

We allow [slow types](https://jsr.io/docs/about-slow-types) in our packages because we use Zod heavily, which relies on type inference.

