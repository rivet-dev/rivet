import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		getRegion: (c) => {
			const region = c.region;
			return region;
		},
	},
});
