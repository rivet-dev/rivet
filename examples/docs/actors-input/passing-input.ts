import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface GameInput {
  gameMode: string;
  maxPlayers: number;
  difficulty?: string;
}

const game = actor({
  createState: (c, input: GameInput) => ({
    gameMode: input.gameMode,
    maxPlayers: input.maxPlayers,
    difficulty: input.difficulty ?? "medium",
  }),
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
