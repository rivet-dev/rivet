import { actor, setup } from "rivetkit";

export type UserSessionPreferences = {
	theme: "light" | "dark";
	language: "en" | "es" | "fr";
};

export type UserSessionActivity = {
	page: string;
	timestamp: number;
};

export type UserSessionState = {
	region: string;
	preferences: UserSessionPreferences;
	recentActivity: UserSessionActivity[];
	lastLoginAt: number;
};

interface UserSessionInput {
	region: string;
}

const MAX_ACTIVITY = 6;

export const userSession = actor({
	// Initialize state with a region-specific input parameter. https://rivet.dev/docs/actors/state
	createState: (_c, input: UserSessionInput): UserSessionState => ({
		region: input.region,
		preferences: {
			theme: "light",
			language: "en",
		},
		recentActivity: [],
		lastLoginAt: Date.now(),
	}),

	actions: {
		// Read session data stored in state.
		getSession: (c) => c.state,

		// Update user preferences stored in state.
		updatePreferences: (
			c,
			preferences: Partial<UserSessionPreferences>,
		) => {
			if (preferences.theme) {
				c.state.preferences.theme = preferences.theme;
			}
			if (preferences.language) {
				c.state.preferences.language = preferences.language;
			}
			return c.state;
		},

		// Log activity and keep a short history of recent pages.
		logActivity: (c, entry: { page: string; isLogin?: boolean }) => {
			const activity: UserSessionActivity = {
				page: entry.page,
				timestamp: Date.now(),
			};
			c.state.recentActivity = [activity, ...c.state.recentActivity].slice(
				0,
				MAX_ACTIVITY,
			);
			if (entry.isLogin) {
				c.state.lastLoginAt = activity.timestamp;
			}
			return c.state;
		},

		// Return the actor's region so the UI can show data locality.
		getRegion: (c) => c.state.region,
	},
});

// Register actors for use in the server.
export const registry = setup({
	use: { userSession },
});
