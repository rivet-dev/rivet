import { actor, queue, setup } from "rivetkit";
import { z } from "zod";

export const worker = actor({
  state: {},
  queues: {
    // Use generic queue typing when you want compile-time typing only.
    foo: queue<{ id: string }, { ok: true }>(),
    // Use schema objects when you want runtime validation for message and completion payloads.
    bar: {
      message: z.object({ id: z.string() }),
      complete: z.object({ ok: z.boolean() }),
    },
  },
});

export const registry = setup({ use: { worker } });
