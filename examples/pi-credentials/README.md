# Pi Credentials

A Pi actor that reads provider logins from a `credentials` actor with `pi({ credentials })`. The `credentials` actor stores one user's logins and refreshes subscription tokens. The Pi actor never receives a refresh token.

## Getting Started

```sh
pnpm install
pnpm --filter @rivet-dev/pi build
cd examples/pi-credentials
pnpm start
```

In another terminal, log in as the user `alice`:

```sh
pnpm login alice
```

Pi actors with key `["alice", ...]` now use Alice's logins.

To use them from an [ACP](https://agentclientprotocol.com) editor such as Zed, add a custom agent server that runs `npx rivet-pi acp --actor agent --user alice --credentials credentials` in this folder. The editor offers the same login.

## Implementation

- [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/pi-credentials/src/actors.ts): the `credentials` actor and the Pi actor.
- [`scripts/login.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/pi-credentials/scripts/login.ts): runs Pi's login in the terminal and saves the result.

Any client that reaches the `credentials` actor can read its tokens. Put it behind [authentication](https://rivet.dev/docs/authentication) before you deploy.

## License

MIT
