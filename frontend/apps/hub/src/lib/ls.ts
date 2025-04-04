import type { AuthContext } from "@/domains/auth/contexts/auth";
import { ls as commonLs } from "@rivet-gg/components";

export const ls = {
	...commonLs,
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
};
