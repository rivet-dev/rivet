import { actor, setup } from "rivetkit";
import { pullQueue } from "./pull-queue";
import { emailQueue } from "./queue";

// docs:start producer-actor
// An actor that produces jobs. Code that called env.EMAIL_QUEUE.send()
// from a Durable Object calls the queue actor's action instead.
const signups = actor({
	actions: {
		register: async (c, email: string) => {
			const client = c.client<typeof registry>();
			await client.emailQueue
				.getOrCreate(["emails"])
				.enqueue({ to: email, subject: "Welcome" });
		},
	},
});
// docs:end producer-actor

export const registry = setup({ use: { emailQueue, pullQueue, signups } });
registry.start();
