import { actor, CreateContextOf, ActionContextOf } from "rivetkit";

const gameRoom = actor({
  onCreate: (c, input: { roomId: string }) => {
    initializeRoom(c, input.roomId);
  },

  createVars: () => ({ players: [] as string[] }),

  actions: {
    addPlayer: (c, playerId: string) => {
      validatePlayer(c, playerId);
      c.vars.players.push(playerId);
    }
  }
});

// Extract CreateContext type for onCreate hook
function initializeRoom(
  context: CreateContextOf<typeof gameRoom>,
  roomId: string
) {
  console.log(`Initializing room: ${roomId}`);
  // context.vars is not available here (not created yet)
}

// Extract ActionContext type for actions
function validatePlayer(
  context: ActionContextOf<typeof gameRoom>,
  playerId: string
) {
  // Full context available in actions
  if (context.vars.players.includes(playerId)) {
    throw new Error("Player already in room");
  }
}
