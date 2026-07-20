import { actor, queue, setup } from "rivetkit";

const worker = actor({
  state: { processed: 0 },
  queues: {
    process: queue<{ taskId: string }>({
      onMessage: async (c, message, { signal }) => {
        await processTask(message.body.taskId, signal);
        c.state.processed += 1;
      },
    }),
  },
});

async function processTask(taskId: string, signal: AbortSignal) {
  await fetch(`https://api.example.com/tasks/${taskId}/complete`, {
    method: "POST",
    signal,
  });
}

export const registry = setup({ use: { worker } });
