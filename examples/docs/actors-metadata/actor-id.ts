import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		getId: (c) => {
			const actorId = c.actorId;
			return actorId;
		},
	},
});
