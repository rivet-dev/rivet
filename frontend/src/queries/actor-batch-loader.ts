import { AsyncBatcher } from "@tanstack/pacer";
import type { Rivet } from "@rivetkit/engine-api-full";

type PendingEntry = {
	resolve: (actor: Rivet.Actor) => void;
	reject: (err: unknown) => void;
};

export function createActorBatchLoader(
	fetchBatch: (actorIds: string[]) => Promise<Rivet.Actor[]>,
) {
	const pending = new Map<string, PendingEntry[]>();

	const batcher = new AsyncBatcher<string>(
		async (actorIds) => {
			const batch = new Map<string, PendingEntry[]>();
			for (const id of actorIds) {
				const entries = pending.get(id);
				if (entries?.length) {
					batch.set(id, entries);
					pending.delete(id);
				}
			}

			try {
				const actors = await fetchBatch(actorIds);
				const actorMap = new Map(actors.map((a) => [a.actorId, a]));
				for (const [id, entries] of batch) {
					const actor = actorMap.get(id);
					for (const { resolve, reject } of entries) {
						if (actor) resolve(actor);
						else reject(new Error(`Actor not found: ${id}`));
					}
				}
			} catch (err) {
				for (const entries of batch.values()) {
					for (const { reject } of entries) reject(err);
				}
			}
		},
		{ wait: 20, maxSize: 32 },
	);

	return {
		load(actorId: string): Promise<Rivet.Actor> {
			return new Promise((resolve, reject) => {
				const entries = pending.get(actorId) ?? [];
				entries.push({ resolve, reject });
				pending.set(actorId, entries);
				batcher.addItem(actorId);
			});
		},
	};
}
