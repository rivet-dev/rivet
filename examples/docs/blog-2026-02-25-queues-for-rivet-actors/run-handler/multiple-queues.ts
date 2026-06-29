import { openai } from "@ai-sdk/openai";
import { generateText } from "ai";
import { actor, queue, setup } from "rivetkit";
import { joinSignals } from "rivetkit/utils";

const agent = actor({
  state: { running: false, messages: [] as string[] },
  queues: {
    // Separate queues for different message types
    prompt: queue<{ prompt: string }, undefined>(),
    stop: queue<{ reason?: string }>(),
  },
  run: async (c) => {
    // Only consume from the prompt queue
    for await (const promptMessage of c.queue.iter({
      names: ["prompt"],
      completable: true,
    })) {
      const stopController = new AbortController();
      const runSignal = joinSignals(c.abortSignal, stopController.signal);

      // Race: watch for stop messages while generating
      c.queue
        .next({ names: ["stop"], signal: runSignal })
        .then((stopMessage) => {
          if (stopMessage) stopController.abort();
        })
        .catch(() => {});

      // Generate until complete or cancelled
      c.state.running = true;
      const { text } = await generateText({
        model: openai("gpt-5"),
        prompt: promptMessage.body.prompt,
        abortSignal: runSignal,
      }).finally(() => {
        stopController.abort();
        c.state.running = false;
      });

      c.state.messages.push(text);
      await promptMessage.complete();
    }
  },
});

export const registry = setup({ use: { agent } });
