import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

type Event = {
  type: string;
  url: string;
};

const hourlyAnalytics = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          type TEXT NOT NULL,
          url TEXT NOT NULL
        );
      `);
    },
  }),
  actions: {
    trackEvent: async (c, event: Event) => {
      await c.db.execute("INSERT INTO events (type, url) VALUES (?, ?)", event.type, event.url);
    },
    getEvents: async (c) => {
      return (await c.db.execute("SELECT type, url FROM events ORDER BY id")) as Event[];
    },
  },
});

export const registry = setup({
  use: { hourlyAnalytics },
});
