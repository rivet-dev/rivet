import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const user = actor({
  state: { name: "" },
  actions: {
    getName: (c) => c.state.name,
  },
});

const registry = setup({ use: { user } });

// docs:start client
// Runs on your backend. The endpoint carries your Rivet token:
// https://my-namespace:<rivet-token>@rivet.example.com
const client = createClient<typeof registry>({
  endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
});
// docs:end client

// docs:start actor
// Defaults to gateway read access, scoped to this Actor's ID.
const handle = client.user.getOrCreate(["user-123"]);
const { token, issuedAt, expiresAt } = await handle.issueToken({
  subject: "user-123",
  expiresIn: 900,
});
// docs:end actor

// docs:start permissions
// Replaces the default. Every grant is scoped to this Actor's ID.
await handle.issueToken({
  subject: "user-123",
  expiresIn: 900,
  permissions: {
    actor_gateway: ["read"],
    actor: ["read", "delete"],
  },
});
// docs:end permissions

// docs:start namespace
// Namespace-wide grants are never added automatically.
await client.auth.issueToken({
  subject: "user-123",
  expiresIn: 900,
  grants: [{ resource: "actor", target: "any", operations: ["create"] }],
});
// docs:end namespace

console.log(token, issuedAt, expiresAt);
