import { createClient } from "rivetkit/client";

const client = createClient();
const chat = client.chatRoom.getOrCreate(["general"], {
  params: { authToken: "jwt-token-here" },
});

// Authentication will happen when calling the action by reading input
// parameters
await chat.sendMessage("Hello, world!");
