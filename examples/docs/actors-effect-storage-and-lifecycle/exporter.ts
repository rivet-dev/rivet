import { Action, Actor } from "@rivetkit/effect";
import { Effect, Scope } from "effect";

export const StartExport = Action.make("StartExport");
export const Pause = Action.make("Pause");
export const Delete = Action.make("Delete");

export const Exporter = Actor.make("Exporter", {
	actions: [StartExport, Pause, Delete],
});

// Stands in for real work.
const runExport = Effect.sleep("30 seconds");

export const ExporterLive = Exporter.toLayer(
	Effect.gen(function* () {
		const wakeScope = yield* Scope.Scope;
		const address = yield* Actor.CurrentAddress;
		yield* Effect.log(`Exporter ${address.key.join("/")} is awake`);

		return Exporter.of({
			/**
			 * Returns at once. The export runs in the background, and the
			 * actor stays awake until it finishes.
			 */
			StartExport: () =>
				runExport.pipe(
					Actor.keepAwake,
					Effect.forkIn(wakeScope),
					Effect.asVoid,
				),
			Pause: () => Actor.sleep,
			Delete: () => Actor.destroy,
		});
	}),
	// docs:start options
	{
		name: "Exporter",
		icon: "file-export",
		sleepTimeout: "5 minutes",
		actionTimeout: "30 seconds",
	},
	// docs:end options
);
