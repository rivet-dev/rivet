import { actor } from "rivetkit";

interface GameInput {
  gameMode: "casual" | "tournament" | "ranked";
  maxPlayers: number;
  difficulty?: "easy" | "medium" | "hard";
}

interface GameState {
  gameMode: string;
  maxPlayers: number;
  difficulty: string;
}

const game = actor({
  createState: (c, input: GameInput): GameState => ({
    gameMode: input.gameMode,
    maxPlayers: input.maxPlayers,
    difficulty: input.difficulty ?? "medium",
  }),

  actions: {
    // Actions are now type-safe
  },
});
