import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { createClient } from "rivetkit/client";

interface ChatRoomInput {
  maxUsers: number;
  isPrivate: boolean;
  moderators: string[];
  settings: { allowImages: boolean; slowMode: boolean };
}

const chatRoom = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS config (id INTEGER PRIMARY KEY CHECK (id = 1), value TEXT NOT NULL)",
      );
    },
  }),
  // Store the input once when the actor is created
  onCreate: async (c, input: ChatRoomInput) => {
    await c.db.execute(
      "INSERT INTO config (id, value) VALUES (1, ?)",
      JSON.stringify(input),
    );
  },
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");
const roomName = "general";

// Create with both key and input
const chatRoomHandle = await client.chatRoom.create(["room", roomName], {
  input: {
    maxUsers: 100,
    isPrivate: false,
    moderators: ["admin1", "admin2"],
    settings: {
      allowImages: true,
      slowMode: false
    }
  }
});
