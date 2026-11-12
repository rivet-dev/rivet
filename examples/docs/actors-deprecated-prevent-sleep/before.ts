import { actor } from "rivetkit";

// An actor that still uses the deprecated `setPreventSleep`
const uploader = actor({
  actions: {
    upload: async (c, url: string) => {
      c.setPreventSleep(true);
      try {
        await fetch(url, { method: "POST" });
      } finally {
        c.setPreventSleep(false);
      }
    },
  },
});
