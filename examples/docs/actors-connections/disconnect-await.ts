import { actor } from "rivetkit";

const myActor = actor({
  actions: {
    disconnect: async (c) => {
      await c.conn.disconnect("Too many requests");
    }
  }
});
