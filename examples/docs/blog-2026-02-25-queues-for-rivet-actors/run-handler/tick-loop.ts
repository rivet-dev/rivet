import { actor, setup } from "rivetkit";
import { interval } from "rivetkit/utils";

const gameRoom = actor({
  state: {
    tick: 0,
    players: {} as Record<string, { x: number; y: number }>,
  },
  // Fixed-interval game loop
  run: async (c) => {
    const tick = interval(100); // 10 ticks per second
    while (!c.aborted) {
      await tick();
      if (c.aborted) break;

      c.state.tick += 1;

      // Update physics, check collisions, etc.
      for (const player of Object.values(c.state.players)) {
        player.x = Math.max(0, Math.min(1000, player.x));
        player.y = Math.max(0, Math.min(1000, player.y));
      }

      c.broadcast("snapshot", c.state);
    }
  },
  actions: {
    setInput: (c, input: { x: number; y: number }) => {
      c.state.players[c.conn.id] = input;
    },
  },
});

export const registry = setup({ use: { gameRoom } });
