import type { AuthContext } from "@/domains/auth/contexts/auth";

type LSKey =
	| `rivet-lastteam-${string}`
	| "rivet-token"
	| "actors-list-preview-width"
	| "actors-list-preview-folded";

export const ls = {
	set: (key: LSKey, value: unknown) => {
		localStorage.setItem(key, JSON.stringify(value));
	},
	get: (key: LSKey) => {
		const value = localStorage.getItem(key);
		try {
			return value ? JSON.parse(value) : null;
		} catch {
			return null;
		}
	},
	remove: (key: LSKey) => {
		localStorage.removeItem(key);
	},
	clear: () => {
		localStorage.clear();
	},

	recentTeam: {
		get: (auth: AuthContext) => {
			return ls.get(
				`rivet-lastteam-${auth.profile?.identity.identityId}`,
			);
		},
		set: (auth: AuthContext, groupId: string) => {
			ls.set(
				`rivet-lastteam-${auth.profile?.identity.identityId}`,
				groupId,
			);
		},
		remove: (auth: AuthContext) => {
			ls.remove(`rivet-lastteam-${auth.profile?.identity.identityId}`);
		},
	},
	actorsList: {
		set: (width: number, folded: boolean) => {
			ls.set("actors-list-preview-width", width);
			ls.set("actors-list-preview-folded", folded);
		},
		getWidth: () => ls.get("actors-list-preview-width"),
		getFolded: () => ls.get("actors-list-preview-folded"),
	},
};
