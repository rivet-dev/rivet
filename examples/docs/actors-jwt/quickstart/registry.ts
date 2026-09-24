import { actor, setup } from "rivetkit";

export const userProfile = actor({
	state: { displayName: "", visits: 0 },

	actions: {
		recordVisit: (c) => {
			c.state.visits += 1;
			return c.state.visits;
		},
		setDisplayName: (c, displayName: string) => {
			c.state.displayName = displayName;
		},
	},
});

export const registry = setup({ use: { userProfile } });
