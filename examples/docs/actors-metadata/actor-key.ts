import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		getKey: (c) => {
			const actorKey = c.key;
			return actorKey;
		},
	},
});
