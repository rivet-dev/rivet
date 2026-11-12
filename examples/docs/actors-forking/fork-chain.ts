import { actor } from "rivetkit";

const document = actor({
	actions: {
		// The actor this one was forked from, or null for an original.
		forkedFrom: (c) => c.forkChain.at(-1)?.actorId ?? null,

		// Every fork behind this actor, oldest first.
		lineage: (c) =>
			c.forkChain.map((link) => ({
				actorId: link.actorId,
				key: link.key,
				forkedAt: new Date(link.forkedAt).toISOString(),
			})),
	},
});
