import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface ChatRoomInput {
  roomName: string;
  isPrivate: boolean;
  maxUsers?: number;
}

// Mock function for demonstration
function setupPrivateRoomLogging(roomName: string) {
  console.log(`Setting up logging for private room: ${roomName}`);
}

const chatRoom = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS room (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          name TEXT NOT NULL,
          is_private INTEGER NOT NULL,
          max_users INTEGER NOT NULL
        );
      `);
      await db.execute(`
        CREATE TABLE IF NOT EXISTS users (
          user_id TEXT PRIMARY KEY
        );
      `);
    },
  }),

  onCreate: async (c, input: ChatRoomInput) => {
    console.log(`Creating room: ${input.roomName}`);

    // Persist the input so actions can read it later
    await c.db.execute(
      "INSERT INTO room (id, name, is_private, max_users) VALUES (1, ?, ?, ?)",
      input?.roomName ?? "Unnamed Room",
      input?.isPrivate ? 1 : 0,
      input?.maxUsers ?? 50,
    );

    // Setup external services based on input
    if (input.isPrivate) {
      setupPrivateRoomLogging(input.roomName);
    }
  },

  actions: {
    // Input remains accessible in actions through SQLite
    getRoomInfo: async (c) => {
      const [room] = (await c.db.execute(
        "SELECT name, is_private, max_users FROM room WHERE id = 1",
      )) as { name: string; is_private: number; max_users: number }[];
      const [users] = (await c.db.execute(
        "SELECT COUNT(*) AS count FROM users",
      )) as { count: number }[];

      return {
        name: room.name,
        isPrivate: room.is_private === 1,
        maxUsers: room.max_users,
        currentUsers: users.count,
      };
    },
  },
});
