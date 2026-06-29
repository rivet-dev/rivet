import { actor, queue, setup } from "rivetkit";

const worker = actor({
  state: { processed: 0 },
  queues: {
    // undefined response type means ack-only, no return data
    process: queue<{ taskId: string }, undefined>(),
  },
  run: async (c) => {
    for await (const message of c.queue.iter({ completable: true })) {
      // Do work
      await processTask(message.body.taskId);
      c.state.processed += 1;
      // Acknowledge completion without returning data
      await message.complete();
    }
  },
});

async function processTask(taskId: string) {
  await fetch(`https://api.example.com/tasks/${taskId}/complete`, {
    method: "POST",
  });
}

export const registry = setup({ use: { worker } });
