import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

type Message = {
  sender: string;
  text: string;
};

// Data actor: handles messages and connections
const chatRoom = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS messages (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          sender TEXT NOT NULL,
          text TEXT NOT NULL
        );
      `);
    },
  }),
  actions: {
    sendMessage: async (c, sender: string, text: string) => {
      const message: Message = { sender, text };
      await c.db.execute("INSERT INTO messages (sender, text) VALUES (?, ?)", sender, text);
      c.broadcast("newMessage", message);
      return message;
    },
    getHistory: async (c) => {
      return (await c.db.execute("SELECT sender, text FROM messages ORDER BY id")) as Message[];
    },
  },
});

// Coordinator: indexes chat rooms
const chatRoomList = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute("CREATE TABLE IF NOT EXISTS chat_rooms (actor_id TEXT PRIMARY KEY)");
    },
  }),
  actions: {
    createChatRoom: async (c, name: string) => {
      const client = c.client<typeof registry>();
      // Create the chat room actor and get its ID
      const handle = await client.chatRoom.create([name]);
      const actorId = await handle.resolve();
      // Track it in the list
      await c.db.execute("INSERT INTO chat_rooms (actor_id) VALUES (?)", actorId);
      return actorId;
    },
    listChatRooms: async (c) => {
      const rows = (await c.db.execute("SELECT actor_id FROM chat_rooms")) as { actor_id: string }[];
      return rows.map((row) => row.actor_id);
    },
  },
});

const registry = setup({
  use: { chatRoom, chatRoomList },
});
