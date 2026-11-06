import { actor } from "rivetkit";

// An actor that still uses the deprecated `c.kv`
const preferences = actor({
  actions: {
    set: async (c, userId: string, theme: string) => {
      await c.kv.put(`theme:${userId}`, theme);
    },
    get: async (c, userId: string) => {
      return await c.kv.get(`theme:${userId}`);
    },
    list: async (c) => {
      return await c.kv.list("theme:");
    },
  },
});
