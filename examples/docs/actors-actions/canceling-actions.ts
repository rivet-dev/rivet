import { actor } from "rivetkit";

const chatActor = actor({
  createVars: () => ({ controller: null as AbortController | null }),

  actions: {
    generate: async (c, prompt: string) => {
      const controller = new AbortController();
      c.vars.controller = controller;
      c.abortSignal.addEventListener("abort", () => controller.abort());

      const response = await fetch("https://api.example.com/generate", {
        method: "POST",
        body: JSON.stringify({ prompt }),
        signal: controller.signal
      });

      return await response.json();
    },

    cancel: (c) => {
      c.vars.controller?.abort();
    }
  }
});
