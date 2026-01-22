import { Effect } from "effect";
import { actor } from "rivetkit";
import { Action, Run, Log, Queue } from "@rivetkit/effect";

interface QueueProcessorState {
	processedMessages: number;
	lastMessageAt: number | null;
	isRunning: boolean;
}

interface Task {
	type: "process" | "compute" | "notify";
	payload: unknown;
}

/**
 * This actor demonstrates:
 * - The `run` handler for background processing
 * - The queues API for task/message processing
 * - Long-running actor patterns
 */
export const queueProcessor = actor({
	state: {
		processedMessages: 0,
		lastMessageAt: null,
		isRunning: false,
	} as QueueProcessorState,

	// The `run` handler is called when the actor starts
	// It's perfect for background task processing loops
	run: Run.effect(function* (c) {
		yield* Log.info("Queue processor starting");

		yield* Action.updateState(c, (s) => {
			s.isRunning = true;
		});

		// Main processing loop
		while (true) {
			// Wait for the next message from the queue
			// This will block until a message is available or the actor is stopped
			const message = yield* Queue.next(c, "tasks", { timeout: 5000 });

			if (!message) {
				yield* Log.debug("No message received, continuing to wait");
				continue;
			}

			yield* Log.info("Processing message", {
				id: message.id,
				name: message.name,
			});

			// Process the task based on its type
			const task = message.body as Task;
			const result = yield* Effect.either(processTask(task));

			if (result._tag === "Right") {
				yield* Log.info("Task processed successfully", {
					type: task.type,
				});
			} else {
				yield* Log.error("Task processing failed", {
					type: task.type,
					error: result.left,
				});
			}

			// Update state
			yield* Action.updateState(c, (s) => {
				s.processedMessages++;
				s.lastMessageAt = Date.now();
			});

			// Broadcast progress to connected clients
			const state = yield* Action.state(c);
			yield* Action.broadcast(c, "progress", {
				processedMessages: state.processedMessages,
				lastMessageAt: state.lastMessageAt,
			});
		}
	}),

	actions: {
		// Get current processor stats
		getStats: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return {
				processedMessages: s.processedMessages,
				lastMessageAt: s.lastMessageAt,
				isRunning: s.isRunning,
			};
		}),

		// Submit a task to the queue
		// In a real app, this would be called by external clients or other actors
		submitTask: Action.effect(function* (c, task: Task) {
			yield* Log.info("Task submitted", { type: task.type });

			// Note: In this example, we can't directly enqueue since enqueue
			// is typically done via the client. This action simulates what
			// an external client would do.
			return { submitted: true, taskType: task.type };
		}),

		// Reset statistics
		resetStats: Action.effect(function* (c) {
			yield* Action.updateState(c, (s) => {
				s.processedMessages = 0;
				s.lastMessageAt = null;
			});
			yield* Log.info("Stats reset");
			return { reset: true };
		}),
	},
});

// Helper function to process different task types
function processTask(task: Task): Effect.Effect<unknown, Error> {
	return Effect.gen(function* () {
		switch (task.type) {
			case "process":
				// Simulate data processing
				yield* Effect.sleep(10);
				return { processed: task.payload };

			case "compute":
				// Simulate computation
				yield* Effect.sleep(20);
				return { computed: true };

			case "notify":
				// Simulate notification
				yield* Effect.sleep(5);
				return { notified: true };

			default:
				return yield* Effect.fail(
					new Error(`Unknown task type: ${task.type}`),
				);
		}
	});
}
