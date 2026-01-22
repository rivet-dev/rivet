import { Data, Effect } from "effect";
import { actor } from "rivetkit";
import { Action, Log } from "@rivetkit/effect";

// Custom error types using Effect's Data.TaggedError
class FetchError extends Data.TaggedError("FetchError")<{
	url: string;
	message: string;
}> {}

class ValidationError extends Data.TaggedError("ValidationError")<{
	field: string;
	message: string;
}> {}

// Simulated external API call
const fetchUserData = (
	userId: string,
): Effect.Effect<{ name: string; email: string }, FetchError> =>
	Effect.tryPromise({
		try: async () => {
			// Simulate API call
			await new Promise((resolve) => setTimeout(resolve, 10));
			return {
				name: `User ${userId}`,
				email: `user${userId}@example.com`,
			};
		},
		catch: () =>
			new FetchError({
				url: `/api/users/${userId}`,
				message: "Failed to fetch user",
			}),
	});

// Simulated notification service
const sendNotification = (
	email: string,
	message: string,
): Effect.Effect<void, FetchError> =>
	Effect.tryPromise({
		try: async () => {
			// Simulate notification
			await new Promise((resolve) => setTimeout(resolve, 10));
		},
		catch: () =>
			new FetchError({
				url: "/api/notify",
				message: "Failed to send notification",
			}),
	});

interface FetchActorState {
	processedUsers: string[];
	lastProcessedAt: number | null;
}

/**
 * This actor demonstrates Effect for multi-step failable operations:
 * - Fetching data from external services
 * - Multi-step workflows with error handling
 * - Actor-to-actor communication
 */
export const fetchActor = actor({
	state: {
		processedUsers: [],
		lastProcessedAt: null,
	} as FetchActorState,

	actions: {
		// Simple Effect-wrapped action
		getStats: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return {
				processedCount: s.processedUsers.length,
				lastProcessedAt: s.lastProcessedAt,
			};
		}),

		// Multi-step workflow with failable operations
		processUser: Action.effect(function* (c, userId: string) {
			yield* Log.info("Starting user processing", { userId });

			// Step 1: Validate input
			if (!userId || userId.length === 0) {
				return yield* Effect.fail(
					new ValidationError({
						field: "userId",
						message: "User ID is required",
					}),
				);
			}

			// Step 2: Fetch user data (can fail)
			yield* Log.info("Fetching user data", { userId });
			const userData = yield* fetchUserData(userId);

			// Step 3: Send notification (can fail)
			yield* Log.info("Sending notification", { email: userData.email });
			yield* sendNotification(
				userData.email,
				`Welcome, ${userData.name}!`,
			);

			// Step 4: Update state
			yield* Action.updateState(c, (s) => {
				s.processedUsers.push(userId);
				s.lastProcessedAt = Date.now();
			});

			yield* Log.info("User processing complete", { userId });

			return {
				success: true,
				user: userData,
			};
		}),

		// Batch processing with error recovery
		processBatch: Action.effect(function* (c, userIds: string[]) {
			const results: Array<{
				userId: string;
				success: boolean;
				error?: string;
			}> = [];

			for (const userId of userIds) {
				// Use Effect.either to handle errors without stopping the batch
				const result = yield* Effect.either(
					Effect.gen(function* () {
						const userData = yield* fetchUserData(userId);
						yield* sendNotification(
							userData.email,
							`Batch welcome, ${userData.name}!`,
						);
						return userData;
					}),
				);

				if (result._tag === "Right") {
					yield* Action.updateState(c, (s) => {
						s.processedUsers.push(userId);
						s.lastProcessedAt = Date.now();
					});
					results.push({ userId, success: true });
				} else {
					results.push({
						userId,
						success: false,
						error: result.left.message,
					});
				}
			}

			return results;
		}),

		// Actor-to-actor communication example
		callQueueProcessor: Action.effect(function* (c, processorKey: string) {
			yield* Log.info("Calling queueProcessor actor", { processorKey });

			// Get the internal client for actor-to-actor communication
			const client = yield* Action.getClient(c);

			// Call another actor's action
			const stats = yield* Effect.promise(async () => {
				const queueProcessor = (client as any).queueProcessor.getOrCreate([
					processorKey,
				]);
				return queueProcessor.getStats();
			});

			yield* Log.info("Got stats from queueProcessor", stats as Record<string, unknown>);

			return { processorKey, stats };
		}),
	},
});
