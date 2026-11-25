import { Action, Actor } from "@rivetkit/effect";
import { Effect, Option, Schema, Scope } from "effect";

export const Watch = Action.make("Watch");

export const CountWatchers = Action.make("CountWatchers", {
	success: Schema.Number,
});

export const Document = Actor.make("Document", {
	actions: [Watch, CountWatchers],
});

export const DocumentLive = Document.toLayer(
	Effect.gen(function* () {
		const watchers = new Set<string>();

		return Document.of({
			// Adds the caller until its connection closes.
			Watch: ({ connection }) =>
				Option.match(connection, {
					onNone: () => Effect.die("Watch needs a connection"),
					onSome: ({ id, scope }) =>
						Effect.acquireRelease(
							Effect.sync(() => {
								watchers.add(id);
							}),
							() =>
								Effect.sync(() => {
									watchers.delete(id);
								}),
						).pipe(Scope.provide(scope)),
				}),
			CountWatchers: () => Effect.sync(() => watchers.size),
		});
	}),
);
