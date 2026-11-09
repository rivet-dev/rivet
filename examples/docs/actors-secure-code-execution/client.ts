import { createClient } from "rivetkit/client";
import type { registry } from "./actor";

const client = createClient<typeof registry>("http://localhost:8080");

// One Actor per agent session, so each session has its own VM.
const sandbox = client.sandbox.getOrCreate(["session-123"]);

const value = await sandbox.run("[1, 2, 3].map((n) => n * 2)");
console.log(value); // [2, 4, 6]
