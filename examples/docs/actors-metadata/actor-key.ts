import { actor } from "rivetkit";

const example = actor({
	actions: {
		getKey: (c) => {
			const actorKey = c.key;
			return actorKey;
		},
	},
});
