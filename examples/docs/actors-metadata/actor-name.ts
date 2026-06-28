import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		getName: (c) => {
			const actorName = c.name;
			return actorName;
		},
	},
});
