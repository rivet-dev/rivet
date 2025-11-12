/** KV keys for using Workers KV to store actor metadata globally. */
export const GLOBAL_KV_KEYS = {
	actorMetadata: (actorId: string): string => {
		return `actor:${actorId}:metadata`;
	},
};
