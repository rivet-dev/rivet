import { createClient } from "rivetkit/client";

const client = createClient();
const chat = client.chatRoom.getOrCreate(["general"], {
  params: { authToken: "jwt-token-here" },
});

const conn = chat.connect();
