import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// Single key: one actor per user
client.user.getOrCreate(["user-123"]);

// Compound key: document scoped to an organization
client.document.getOrCreate(["org-acme", "doc-456"]);
