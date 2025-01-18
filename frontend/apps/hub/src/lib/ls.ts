import type { AuthContext } from "@/domains/auth/contexts/auth";

type LSKey = `rivet-lastteam-${string}` | "rivet-token";

export const ls = {
	set: (key: LSKey, value: unknown) => {
		localStorage.setItem(key, JSON.stringify(value));
	},
	get: (key: LSKey) => {
		const value = localStorage.getItem(key);
		return value ? JSON.parse(value) : null;
	},
	remove: (key: LSKey) => {
		localStorage.removeItem(key);
	},
	clear: () => {
		localStorage.clear();
	},

	recentTeam: {
		set: (auth: AuthContext, groupId: string) => {
			localStorage.setItem(
				`rivet-lastteam-${auth.profile?.identity.identityId}`,
				groupId,
			);
		},
		remove: (auth: AuthContext) => {
			localStorage.removeItem(
				`rivet-lastteam-${auth.profile?.identity.identityId}`,
			);
		},
	},
};
