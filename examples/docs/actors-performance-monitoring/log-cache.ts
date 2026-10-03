import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const myActor = actor({
  db: db(),
  actions: {
    logCache: async (c) => {
      await c.db.execute("SELECT 1");
      const metrics = await c.db.nativeMetrics?.();
      if (metrics) {
        c.log.info({ msg: "SQLite cache", ...metrics });
      }
    },
  },
});
