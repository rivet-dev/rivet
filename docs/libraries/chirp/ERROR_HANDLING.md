# Error Handling

## Error registry

All recoverable errors are configured in [`errors/`](../../../errors).

See the [frontmatter](https://jekyllrb.com/docs/front-matter/) config at [`lib/formatted-error/build.rs`](../../../lib/formatted-error/build.rs).

These get auto-generated to https://rivet.gg/docs.

## Service types

### Operations: Throw errors normally

Throw errors using the macros defined in [`lib/global-error/src/macros.rs`](../../../lib/global-error/src/macros.rs).

`err_code!` should be used for any potential user error. This throws errors from the error registry.

`bail!` and `ensure!` should be used like a safe alternative to the `panic!` and `assert!` macro.

### Consumers: Do or die

Consumers will be retried until they succeed without an error. Therefore, errors should only be returned if retrying at a later date will work.

If an error does need to be handled explicitly by another service, publish a separate message for dispatching error events (i.e. a consumer of `msg-yak-shake` will produce on error `msg-yak-shave-fail`).

It's common for consumers to have a separate validation service, e.g. `game-version-create` has a separate `game-version-validate` service.
