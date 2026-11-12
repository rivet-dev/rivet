import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface GameInput {
  gameMode: "casual" | "tournament" | "ranked";
  maxPlayers: number;
  difficulty?: "easy" | "medium" | "hard";
}

const game = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS game (id INTEGER PRIMARY KEY CHECK (id = 1), game_mode TEXT NOT NULL, max_players INTEGER NOT NULL, difficulty TEXT NOT NULL)",
      );
    },
  }),

  // The input type is checked wherever the actor is created
  onCreate: async (c, input: GameInput) => {
    await c.db.execute(
      "INSERT INTO game (id, game_mode, max_players, difficulty) VALUES (1, ?, ?, ?)",
      input.gameMode,
      input.maxPlayers,
      input.difficulty ?? "medium",
    );
  },

  actions: {
    // Actions are now type-safe
  },
});
