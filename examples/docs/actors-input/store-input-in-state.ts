import { actor } from "rivetkit";

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

interface GameState {
  config: GameConfig;
  players: Record<string, boolean>;
  gameState: string;
}

const game = actor({
  createState: (c, input: GameInput): GameState => ({
    // Store input configuration in state
    config: {
      gameMode: input.gameMode,
      maxPlayers: input.maxPlayers,
      difficulty: input?.difficulty ?? "medium",
    },
    // Runtime state
    players: {},
    gameState: "waiting",
  }),

  actions: {
    getConfig: (c) => c.state.config,
    updateDifficulty: (c, difficulty: string) => {
      c.state.config.difficulty = difficulty;
    },
  },
});
