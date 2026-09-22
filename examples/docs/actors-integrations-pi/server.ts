import { pi } from "@rivet-dev/pi";
import { setup } from "rivetkit";

const agent = pi({
	actions: {
		ping: () => "pong",
	},
});

export const registry = setup({
	use: { agent },
});

registry.start();
