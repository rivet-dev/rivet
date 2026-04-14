import { actor } from "rivetkit";

export const warmupActor = actor({
	actions: {
		ping: () => true,
	},
});
