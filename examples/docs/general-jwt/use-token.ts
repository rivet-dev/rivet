import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const user = actor({
  state: { name: "" },
  actions: {
    getName: (c) => c.state.name,
  },
});

const registry = setup({ use: { user } });

// Runs in the browser. The endpoint carries only the namespace; the token comes from your backend.
const client = createClient<typeof registry>({
  endpoint: "https://my-namespace@rivet.example.com",
  // Called on first use and again when the token expires.
  getToken: async () => {
    const response = await fetch("/api/rivet-token", { method: "POST" });
    const { token } = (await response.json()) as { token: string };
    return token;
  },
});

const name = await client.user.get(["user-123"]).getName();
console.log(name);
