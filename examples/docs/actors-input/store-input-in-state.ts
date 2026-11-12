import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface GameInput {
  gameMode: string;
  maxPlayers: number;
  difficulty?: string;
}

interface GameConfig {
  gameMode: string;
  maxPlayers: number;
  difficulty: string;
}

const game = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS config (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          game_mode TEXT NOT NULL,
          max_players INTEGER NOT NULL,
          difficulty TEXT NOT NULL
        );
      `);
    },
  }),

  // onCreate runs once, so store the input configuration in SQLite
  onCreate: async (c, input: GameInput) => {
    await c.db.execute(
      "INSERT INTO config (id, game_mode, max_players, difficulty) VALUES (1, ?, ?, ?)",
      input.gameMode,
      input.maxPlayers,
      input?.difficulty ?? "medium",
    );
  },

  actions: {
    getConfig: async (c): Promise<GameConfig> => {
      const [row] = (await c.db.execute(
        "SELECT game_mode, max_players, difficulty FROM config WHERE id = 1",
      )) as { game_mode: string; max_players: number; difficulty: string }[];
      return {
        gameMode: row.game_mode,
        maxPlayers: row.max_players,
        difficulty: row.difficulty,
      };
    },
    updateDifficulty: async (c, difficulty: string) => {
      await c.db.execute("UPDATE config SET difficulty = ? WHERE id = 1", difficulty);
    },
  },
});
