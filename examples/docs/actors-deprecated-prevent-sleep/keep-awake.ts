import { actor } from "rivetkit";

const uploader = actor({
  actions: {
    // `keepAwake` blocks sleep until the promise settles and returns its result
    upload: async (c, url: string) => {
      const response = await c.keepAwake(fetch(url, { method: "POST" }));
      return response.status;
    },

    // `waitUntil` lets the action return now and finishes the work in the
    // background before the actor sleeps
    uploadInBackground: (c, url: string) => {
      c.waitUntil(fetch(url, { method: "POST" }));
    },
  },
});
