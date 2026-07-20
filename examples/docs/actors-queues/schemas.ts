import { actor, queue, setup } from "rivetkit";
import { z } from "zod";

export const worker = actor({
  state: {},
  queues: {
    // Use generic queue typing when you want compile-time typing only.
    foo: queue<{ id: string }>(),
    // Use schema objects when you want runtime validation for messages.
    bar: {
      message: z.object({ id: z.string() }),
    },
  },
});

export const registry = setup({ use: { worker } });
