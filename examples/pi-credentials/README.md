# Pi Credentials

Let each user run Pi on their own Claude or ChatGPT subscription, or their own API key, instead of yours.

Pi logs in to subscriptions with headless OAuth. The user opens a link, approves, and pastes back a code or enters a device code. This example stores each user's logins in a `credentials` actor and gives them to the Pi actor with `pi({ credentials })`. The `credentials` actor refreshes subscription tokens, so the Pi actor never receives a refresh token.

## Getting Started

```sh
pnpm install
pnpm --filter @rivet-dev/pi build
cd examples/pi-credentials
pnpm start
```

In another terminal, log in to a provider as the user `alice`:

```sh
pnpm provider-login alice
```

Pi actors with key `["alice", ...]` now use Alice's logins.

`provider-login` is only for trying the example. Your app needs its own login flow, such as a settings page in your web app, that saves the result with the `credentials` actor's `save` action.

## Implementation

- [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/pi-credentials/src/actors.ts): the `credentials` actor, which saves, lists, reads, and refreshes a user's logins, and the Pi actor that reads from it.
- [`scripts/provider-login.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/pi-credentials/scripts/provider-login.ts): runs Pi's headless login in the terminal and saves the result.

Any client that reaches the `credentials` actor can read its tokens. Put it behind [authentication](https://rivet.dev/docs/authentication) before you deploy.

## License

MIT
