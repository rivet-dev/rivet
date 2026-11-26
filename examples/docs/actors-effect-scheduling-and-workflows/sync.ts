import { Workflow } from "@rivetkit/effect";
import { Effect } from "effect";

// Stand-ins for calls to another system.
const readPageSize = Effect.succeed(100);
const fetchPage = (page: number, pageSize: number) =>
	Effect.succeed(page < 3 ? new Array<string>(pageSize).fill("item") : []);
const notifyOwner = (count: number) => Effect.log(`Synced ${count} items`);

export const Sync = Workflow.make("Sync");

export const SyncLive = Sync.toLayer(
	Effect.succeed(
		Effect.gen(function* () {
			// Saved with the next write, so a crash can run it again.
			const pageSize = yield* Workflow.step(
				"read-page-size",
				readPageSize,
				{ ephemeral: true },
			);

			// RivetKit saves the state after each iteration.
			const count = yield* Workflow.loop("pages", {
				state: { page: 0, count: 0 },
				run: ({ page, count }) =>
					Effect.gen(function* () {
						const items = yield* Workflow.step(
							"fetch",
							fetchPage(page, pageSize),
						);
						if (items.length === 0)
							return Workflow.Loop.break(count);
						return Workflow.Loop.continue({
							page: page + 1,
							count: count + items.length,
						});
					}),
			});

			// Only a timeout comes back as a value. Other failures fail the workflow.
			yield* Workflow.tryStep("notify", notifyOwner(count), {
				timeout: "10 seconds",
				catch: ["timeout"],
			}).pipe(
				Effect.catchTag("WorkflowStepFailure", () =>
					Effect.logWarning("The owner was not notified"),
				),
			);
		}),
	),
	{
		onError: (event) =>
			"step" in event
				? Effect.logWarning(
						`Step ${event.step.stepName} failed: ${event.step.error.message}`,
					)
				: Effect.void,
	},
);
