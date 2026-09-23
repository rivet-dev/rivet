# Scoped JWT counter

A backend authenticates a user, creates a private counter actor, and issues a short-lived JWT that grants access only to that actor's gateway. The RivetKit client renews the JWT through `getToken` and never receives the Engine admin token.

## Prerequisites

Configure an Engine with `auth.admin_token` and JWT issuance enabled. Set these variables for the backend:

| Variable | Purpose |
| --- | --- |
| `RIVET_ENDPOINT` | Engine API URL, such as `http://127.0.0.1:6420` |
| `RIVET_NAMESPACE` | Namespace containing the counter actor |
| `RIVET_ADMIN_TOKEN` | Engine admin token; **backend only** |
| `DEMO_USER`, `DEMO_PASSWORD` | Local example login |

## Getting Started

Run `pnpm --filter jwt-counter-example start`. In another terminal, set the same endpoint, namespace, and demo login variables, then run `pnpm --filter jwt-counter-example client`. The client process does not need `RIVET_ADMIN_TOKEN`.

With the backend and Engine running, use `pnpm --filter jwt-counter-example smoke` to check issuance, scoped access, renewal after a rejected token, and no replay of the rejected action.

## Features

- The backend keeps the admin token private and issues a 30-second actor-specific JWT.
- RivetKit's `getToken` callback renews the client credential.
- An invalid JWT fails closed, and the next action requests a fresh token.

## Implementation

The [backend](./src/server.ts) checks the demo login, creates the actor, and calls `/auth/tokens`. The [client](./src/client.ts) uses only the scoped JWT. The [smoke test](./tests/smoke.ts) exercises expiry recovery and rejection.

`DEMO_ISSUER_URL` defaults to `http://127.0.0.1:3020`. Basic authentication is only for this CLI example. Use HTTPS outside loopback and replace Basic with your application's login/session middleware in a real app. Never put the admin token, demo password, or a signing key in a client bundle.

## Resources

- [Rivet documentation](https://rivet.dev/docs)

## License

MIT
