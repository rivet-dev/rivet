# Better Auth + user actors

A small React page demonstrates signup and login with Better Auth. Each user gets a separate `user` actor containing a counter, accessed through the normal RivetKit SDK. Better Auth owns the login session; RivetKit's `getToken` callback obtains short-lived actor tokens using that session. No RivetKit React SDK is needed.

For a token-only example without user authentication, see [jwt-counter](../jwt-counter).

## Getting started

Use Node.js 22.13 or later and an Engine with JWT issuance enabled. Set the Engine endpoint URL, which includes the namespace and token, and configure Better Auth:

```sh
export RIVET_ENDPOINT="http://default:YOUR_TOKEN@127.0.0.1:6420"
export BETTER_AUTH_URL="http://localhost:5173"
export BETTER_AUTH_SECRET="$(openssl rand -hex 32)"
pnpm --filter jwt-better-auth-example dev --port 5173 --strictPort
```

Open `http://localhost:5173` and create an account, or log in with **alice@example.com** / **demo-password**. These public demo credentials are also shown below the form. Click **Increment**, wait a minute, and click again: actor access renews without another login.

`BETTER_AUTH_URL` must match the URL you open in the browser. To open the example from another device, use a reachable hostname or IP in both URLs and add `--host 0.0.0.0` to the dev command. Keep `RIVET_ENDPOINT` and `BETTER_AUTH_SECRET` on the server; never prefix them with `VITE_`. URL-encode special characters in the Engine credentials.

The server automatically creates Better Auth's SQLite tables and the hardcoded demo account. Users and sessions are stored locally in `.data/auth.sqlite`; actor state stays in Rivet. Keep `BETTER_AUTH_SECRET` stable across restarts. This shared demo account is for examples only; use your own accounts in an application.

If another local Services process already owns port 8642, set `RIVET_RUN_SERVICES=0`. This example does not use Services.

Vite serves React and the backend from the same origin. To serve a built version with the same environment:

```sh
pnpm --filter jwt-better-auth-example build
pnpm --filter jwt-better-auth-example start --port 5173
```

Use HTTPS outside local development.

## How it works

1. [`src/auth.ts`](./src/auth.ts) configures Better Auth's email/password login and seeds the account from [`demo-account.ts`](./demo-account.ts).
2. [`frontend/login.ts`](./frontend/login.ts) calls Better Auth to log in. The browser receives an HTTP-only session cookie, then asks the backend for the user actor ID.
3. [`frontend/App.tsx`](./frontend/App.tsx) creates a normal RivetKit client with `getToken`, then connects to the user actor.
4. [`src/server.ts`](./src/server.ts) verifies the Better Auth session before returning the user actor or calling `user.issueToken({ subject: userId, expiresIn: 30 })`. The helper defaults to `actor_gateway: ["read"]` for that actor. The user ID comes from the verified session, not from the browser.
5. When the actor token expires, RivetKit requests a fresh token and reconnects. The user stays logged in while the Better Auth session is valid.

The client setup is:

```ts
const client = createClient<typeof registry>({
  endpoint,
  namespace,
  getToken: getActorToken,
});
const user = client.user.getForId(actorId).connect();
await user.increment(1);
```

`getActorToken` calls `/api/token`, and the browser automatically sends the session cookie. The endpoint always issues a fresh token, so the callback does not need to inspect `forceRefresh`. RivetKit caches tokens itself. Failed actor actions are not automatically replayed; the page lets you try again.

Logging out calls Better Auth's sign-out endpoint and closes the actor connection. It prevents further token issuance for that session, but an already-issued actor token remains valid until expiry. If the login session expires, the page asks you to log in again. The counter is preserved across logins. Different accounts get different actors; neither can access the other's actor.

The `actor_gateway` grant's `read` operation permits gateway access, including mutating actions such as `increment`. Engine allows 30 seconds of clock skew after a JWT's expiry. Metadata lookup uses RivetKit's defaults.

## Verification

With the server running and its Engine environment configured, run:

```sh
pnpm --filter jwt-better-auth-example check-types
pnpm --filter jwt-better-auth-example smoke
```

The smoke test checks signup, login, session cookies, separate user actors, the optional CLI, actor isolation, token expiry and renewal, no action replay, and rejection of the saved session cookie after logout. It takes about 70 seconds. Set `DEMO_ISSUER_URL` if the app is not at `http://localhost:5173`.

[`scripts/client.ts`](./scripts/client.ts) is an optional Node version of the flow. It forwards cookies explicitly because Node has no browser cookie jar.

## Resources

- [Rivet documentation](https://rivet.dev/docs)
- [Better Auth Hono integration](https://better-auth.com/docs/integrations/hono)

## License

MIT
