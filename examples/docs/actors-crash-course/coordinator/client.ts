import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// Coordinator per org
const coordinator = client.chatRoomList.getOrCreate(["org-acme"]);
await coordinator.addRoom("general");
await coordinator.addRoom("random");

// Access chat rooms created by coordinator
client.chatRoom.get(["org-acme", "general"]);
