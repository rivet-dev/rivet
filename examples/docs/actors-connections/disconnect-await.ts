import { actor } from "rivetkit";

const myActor = actor({
  state: {},
  actions: {
    disconnect: async (c) => {
      await c.conn.disconnect("Too many requests");
    }
  }
});
