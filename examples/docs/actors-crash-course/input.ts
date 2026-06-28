import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const game = actor({
	state: { mode: "" },
	createState: (c, input: { mode: string }) => ({
		mode: input.mode, // Store input in state for later access
	}),
	actions: {
		getMode: (c) => c.state.mode,
	},
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>("http://localhost:6420");

// Client usage
const gameHandle = client.game.getOrCreate(["game-1"], {
	createWithInput: { mode: "ranked" },
});
