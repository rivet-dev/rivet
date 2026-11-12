import { actor } from "rivetkit";

const example = actor({
	actions: {
		getRegion: (c) => {
			const region = c.region;
			return region;
		},
	},
});
