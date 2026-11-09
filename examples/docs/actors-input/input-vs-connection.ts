import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { createClient } from "rivetkit/client";

interface RoomInput { roomName: string; isPrivate: boolean; }
interface ConnParams { userId: string; displayName: string; }

const chatRoom = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS room (id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT NOT NULL, is_private INTEGER NOT NULL)",
      );
    },
  }),
  // Input configures the whole actor once, when it is created
  onCreate: async (c, input: RoomInput) => {
    await c.db.execute(
      "INSERT INTO room (id, name, is_private) VALUES (1, ?, ?)",
      input.roomName,
      input.isPrivate ? 1 : 0,
    );
  },
  // Connection parameters describe each client that connects
  createVars: () => ({ users: new Map<string, ConnParams>() }),
  onBeforeConnect: (c, params: ConnParams) => {
    if (!params.userId) throw new Error("Missing userId");
  },
  onConnect: (c, conn) => {
    c.vars.users.set(conn.id, conn.params);
  },
  onDisconnect: (c, conn) => {
    c.vars.users.delete(conn.id);
  },
  actions: {}
});

const registry = setup({ use: { chatRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

// Actor creation with input
const room = await client.chatRoom.create(["room-123"], {
  input: {
    roomName: "General Discussion",
    isPrivate: false,
  },
});
