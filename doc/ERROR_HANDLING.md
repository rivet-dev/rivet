# Error Handling

## RPC: Validate then run

Errors shown to the client should be done through a validation step.

If a service returns a bad request, that should be treated as an internal error to the user since the program did not validate their input correctly.

## Consumers: Do or die

Consumers will be retried until they succeed without an error. Therefore, errors
should only be returned if retrying at a later date will work.

If an error does need to be handled explicitly by another service, publish a
separate message for dispatching error events (i.e. a consumer of
`msg-yak-shake` will produce on error `msg-yak-shave-fail`).

