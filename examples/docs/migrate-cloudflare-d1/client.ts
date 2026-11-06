import { createClient } from "rivetkit/client";
import type { registry } from "./define";

const client = createClient<typeof registry>("http://localhost:6420");

// Each tenant gets its own actor, and so its own database. The actor and its
// tables are created the first time the key is used.
const acme = client.tenantDb.getOrCreate(["acme"]);

await acme.addUser("ada@example.com", "Ada");
const users = await acme.listUsers();
console.log(users);
