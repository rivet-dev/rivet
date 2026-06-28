import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// Compound key: [org, room]
client.chatRoom.getOrCreate(["org-acme", "general"]);
