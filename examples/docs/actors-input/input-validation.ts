import { actor } from "rivetkit";
import { z } from "zod";

const GameInputSchema = z.object({
  gameMode: z.enum(["casual", "tournament", "ranked"]),
  maxPlayers: z.number().min(2).max(16),
  difficulty: z.enum(["easy", "medium", "hard"]).optional(),
});

type GameInput = z.infer<typeof GameInputSchema>;

interface GameState {
  gameMode: string;
  maxPlayers: number;
  difficulty: string;
  players: Record<string, boolean>;
  gameState: string;
}

const game = actor({
  createState: (c, inputRaw: GameInput): GameState => {
    // Validate input
    const input = GameInputSchema.parse(inputRaw);

    return {
      gameMode: input.gameMode,
      maxPlayers: input.maxPlayers,
      difficulty: input.difficulty ?? "medium",
      players: {},
      gameState: "waiting",
    };
  },

  actions: {
    // Actions can access the validated input via state
    getGameInfo: (c) => ({
      gameMode: c.state.gameMode,
      maxPlayers: c.state.maxPlayers,
      difficulty: c.state.difficulty,
      currentPlayers: Object.keys(c.state.players).length,
    }),
  },
});
