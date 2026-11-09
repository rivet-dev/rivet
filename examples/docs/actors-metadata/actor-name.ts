import { actor } from "rivetkit";

const example = actor({
	actions: {
		getName: (c) => {
			const actorName = c.name;
			return actorName;
		},
	},
});
