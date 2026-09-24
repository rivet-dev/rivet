# Scoped JWT counter

The smallest JWT flow: a server issues a token scoped to one counter actor, and a React page uses the normal RivetKit SDK to connect and increment it. There is no signup or login. For signup and login, see [jwt-better-auth](../jwt-better-auth).

## Getting started

Use an Engine with JWT issuance enabled. Set its endpoint URL, which includes the namespace and token:

```sh
export RIVET_ENDPOINT="http://default:YOUR_TOKEN@127.0.0.1:6420"
pnpm --filter jwt-counter-example dev
```

Open Vite's URL, normally `http://localhost:5173`, and click **Connect to counter**, then **Increment**.

The configured Engine endpoint stays on the server. The page receives only a token for the shared demo counter. The public `/api/token` endpoint deliberately allows anyone to access this counter; it is not a user authentication system.

The browser connects directly to the Engine endpoint. Use a reachable hostname or IP to access it from another device, and start Vite with `dev --host 0.0.0.0`. URL-encode special characters in the Engine credentials. Use HTTPS outside local development.

If another local Services process owns port 8642, set `RIVET_RUN_SERVICES=0`. This example does not use Services.

## Read the flow

1. [`src/server.ts`](./src/server.ts) returns the demo counter’s connection details from `/api/counter` and calls `counter.issueToken({ expiresIn: 30 })` in `/api/token`. The actor helper defaults to the `actor_gateway: ["read"]` permission for that counter only.
2. [`frontend/App.tsx`](./frontend/App.tsx) loads the connection details and lets RivetKit request tokens:

   ```ts
   const client = createClient<typeof registry>({
     endpoint,
     namespace,
     getToken: async () => (await requestToken()).token,
   });
   const counter = client.counter.getForId(actorId).connect();
   await counter.increment(1);
   ```

3. [`src/actors.ts`](./src/actors.ts) defines the counter and its actions.

Tokens last **30 seconds**. The page uses `getToken` to fetch the first token and every subsequent token whenever RivetKit requests it. Watch **Refreshes** increase while the counter stays connected. Engine allows an additional 30 seconds of clock skew on an existing WebSocket, so renewal occurs at about **60 seconds**. There is no page reload or custom refresh timer. [jwt-better-auth](../jwt-better-auth) adds a login session to authorize token issuance.

Metadata lookup uses RivetKit's defaults. The `actor_gateway` grant's `read` operation allows gateway access, including mutating actions such as `increment`.

## Build and verify

```sh
pnpm --filter jwt-counter-example check-types
pnpm --filter jwt-counter-example build
pnpm --filter jwt-counter-example start --port 5173
```

With the server running and `RIVET_ENDPOINT` configured, run `pnpm --filter jwt-counter-example smoke`. It checks token issuance, the CLI, actor isolation, expiry, renewal, and no action replay; it takes about 70 seconds. Set `DEMO_ISSUER_URL` if the app is not at `http://localhost:5173`.

[`scripts/client.ts`](./scripts/client.ts) demonstrates the same flow from Node.

## License

MIT
