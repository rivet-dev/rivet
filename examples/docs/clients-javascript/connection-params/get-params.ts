import { createClient } from "rivetkit/client";

async function getAuthToken(): Promise<string> {
  return "jwt-token-here";
}

const client = createClient();
const chat = client.chatRoom.getOrCreate(["general"], {
  getParams: async () => ({
    authToken: await getAuthToken(),
  }),
});

const conn = chat.connect();
