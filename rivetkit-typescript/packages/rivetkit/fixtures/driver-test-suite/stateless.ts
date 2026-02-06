import { actor } from "rivetkit";

// Actor without state - only has actions
export const statelessActor = actor({
	actions: {
		ping: () => "pong",
		echo: (c, message: string) => message,
		getActorId: (c) => c.actorId,
		// Try to access state - should throw StateNotEnabled
		tryGetState: (c) => {
			try {
				// State is typed as undefined, but we want to test runtime behavior
				const state = c.state;
				return { success: true, state };
			} catch (error) {
				return { success: false, error: (error as Error).message };
			}
		},
		// Try to access db - should throw DatabaseNotEnabled
		tryGetDb: (c) => {
			try {
				// DB is typed as undefined, but we want to test runtime behavior
				const db = c.db;
				return { success: true, db };
			} catch (error) {
				return { success: false, error: (error as Error).message };
			}
		},
	},
});
