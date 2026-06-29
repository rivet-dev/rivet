import { createClient } from "rivetkit/client";

// This only works for stateless actions, not WebSockets
const client = createClient({
  headers: {
    Authorization: "Bearer my-token",
  },
});

const chat = client.chatRoom.getOrCreate(["general"]);

// Authentication will happen when calling the action by reading headers
await chat.sendMessage("Hello, world!");
