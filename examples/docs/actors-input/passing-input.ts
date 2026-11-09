import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { createClient } from "rivetkit/client";

interface GameInput {
  gameMode: string;
  maxPlayers: number;
  difficulty?: string;
}

const game = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS game (id INTEGER PRIMARY KEY CHECK (id = 1), game_mode TEXT NOT NULL, max_players INTEGER NOT NULL, difficulty TEXT NOT NULL)",
      );
    },
  }),
  onCreate: async (c, input: GameInput) => {
    await c.db.execute(
      "INSERT INTO game (id, game_mode, max_players, difficulty) VALUES (1, ?, ?, ?)",
      input.gameMode,
      input.maxPlayers,
      input.difficulty ?? "medium",
    );
  },
  actions: {}
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>("http://localhost:6420");

// Client side - create with input
const gameHandle = await client.game.create(["game-123"], {
  input: {
    gameMode: "tournament",
    maxPlayers: 8,
    difficulty: "hard",
  }
});

// getOrCreate can also accept input (used only if creating)
const gameHandle2 = client.game.getOrCreate(["game-456"], {
  createWithInput: {
    gameMode: "casual",
    maxPlayers: 4,
  }
});
