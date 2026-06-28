import { actor, setup } from "rivetkit";

const chatRoom = actor({
	state: {
		messages: [],
	},

	actions: {
		// Get actor metadata
		getMetadata: (c) => {
			return {
				actorId: c.actorId,
				name: c.name,
				key: c.key,
				region: c.region,
			};
		},
	},
});

export const registry = setup({
	use: { chatRoom },
});

registry.start();
