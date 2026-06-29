import { actor } from "rivetkit";

const chatActor = actor({
  actions: {
    generate: async (c, prompt: string) => {
      const response = await fetch("https://api.example.com/generate", {
        method: "POST",
        body: JSON.stringify({ prompt }),
        signal: c.abortSignal
      });

      return await response.json();
    }
  }
});
