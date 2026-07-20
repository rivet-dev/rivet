import { openai } from "@ai-sdk/openai";
import { generateText } from "ai";
import { actor, queue, setup } from "rivetkit";
import { joinSignals } from "rivetkit/utils";

export const agent = actor({
  state: { running: false, messages: [] as string[] },
  queues: {
    prompt: queue<{ prompt: string }>(),
    stop: queue<{ reason?: string }>(),
  },
  run: async (c) => {
    for await (const promptMessage of c.queue.iter({ names: ["prompt"] })) {
      // Create a stop controller for this prompt run.
      const stopController = new AbortController();
      const runSignal = joinSignals(c.abortSignal, stopController.signal);

      // Watch for stop messages while generation is running.
      const stopWatcher = c.queue
        .next({ names: ["stop"], signal: runSignal })
        .then((stopMessage) => {
          if (stopMessage) stopController.abort();
        })
        .catch(() => {});

      // Generate a response for the prompt.
      c.state.running = true;
      const { text } = await generateText({
        model: openai("gpt-5"),
        prompt: promptMessage.body.prompt,
        abortSignal: runSignal,
      }).finally(async () => {
        stopController.abort();
        c.state.running = false;
      });

      // Append each model response to actor state. Raw pulls consume before return.
      c.state.messages.push(text);
    }
  },
});

export const registry = setup({ use: { agent } });
