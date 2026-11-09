import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import { z } from "zod";

const GameInputSchema = z.object({
  gameMode: z.enum(["casual", "tournament", "ranked"]),
  maxPlayers: z.number().min(2).max(16),
  difficulty: z.enum(["easy", "medium", "hard"]).optional(),
});

type GameInput = z.infer<typeof GameInputSchema>;

const game = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS game (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          game_mode TEXT NOT NULL,
          max_players INTEGER NOT NULL,
          difficulty TEXT NOT NULL
        );
      `);
      await db.execute(`
        CREATE TABLE IF NOT EXISTS players (
          player_id TEXT PRIMARY KEY
        );
      `);
    },
  }),

  onCreate: async (c, inputRaw: GameInput) => {
    // Validate input
    const input = GameInputSchema.parse(inputRaw);

    await c.db.execute(
      "INSERT INTO game (id, game_mode, max_players, difficulty) VALUES (1, ?, ?, ?)",
      input.gameMode,
      input.maxPlayers,
      input.difficulty ?? "medium",
    );
  },

  actions: {
    // Actions read the validated input back from SQLite
    getGameInfo: async (c) => {
      const [info] = (await c.db.execute(
        "SELECT game_mode, max_players, difficulty FROM game WHERE id = 1",
      )) as { game_mode: string; max_players: number; difficulty: string }[];
      const [players] = (await c.db.execute(
        "SELECT COUNT(*) AS count FROM players",
      )) as { count: number }[];

      return {
        gameMode: info.game_mode,
        maxPlayers: info.max_players,
        difficulty: info.difficulty,
        currentPlayers: players.count,
      };
    },
  },
});
