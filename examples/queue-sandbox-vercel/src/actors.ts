import { setup } from "rivetkit";
import { sender } from "./actors/sender.ts";
import { multiQueue } from "./actors/multi-queue.ts";
import { timeout } from "./actors/timeout.ts";
import { worker } from "./actors/worker.ts";
import { selfSender } from "./actors/self-sender.ts";
import { keepAwake } from "./actors/keep-awake.ts";

export const registry = setup({
	use: {
		sender,
		multiQueue,
		timeout,
		worker,
		selfSender,
		keepAwake,
	},
});
