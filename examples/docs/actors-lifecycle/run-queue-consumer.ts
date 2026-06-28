import { actor } from "rivetkit";

// Example: Queue consumer
const queueConsumer = actor({
  state: { processedCount: 0 },

  run: async (c) => {
    c.log.info("Queue consumer started");

    while (!c.aborted) {
      // Wait for next message with timeout.
      const message = await c.queue.next({ names: ["tasks"], timeout: 1000 });

      if (message) {
        c.log.info({ msg: "processing message", body: message.body });
        // Process the message...
        c.state.processedCount++;
      }
    }

    c.log.info("Queue consumer exiting gracefully");
  },

  actions: {
    getProcessedCount: (c) => c.state.processedCount
  }
});
