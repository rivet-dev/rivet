import { actor, CreateContextOf, ActionContextOf } from "rivetkit";

const gameRoom = actor({
  createState: (c, input: { roomId: string }): { players: string[]; score: number } => {
    initializeRoom(c, input.roomId);
    return { players: [] as string[], score: 0 };
  },

  actions: {
    addPlayer: (c, playerId: string) => {
      validatePlayer(c, playerId);
      c.state.players.push(playerId);
    }
  }
});

// Extract CreateContext type for createState hook
function initializeRoom(
  context: CreateContextOf<typeof gameRoom>,
  roomId: string
) {
  console.log(`Initializing room: ${roomId}`);
  // context.state is not available here (being created)
  // context.vars is not available here (not created yet)
}

// Extract ActionContext type for actions
function validatePlayer(
  context: ActionContextOf<typeof gameRoom>,
  playerId: string
) {
  // Full context available in actions
  if (context.state.players.includes(playerId)) {
    throw new Error("Player already in room");
  }
}
