import { Action, Actor, Event } from "@rivetkit/effect";
import { Effect, Option, Schema } from "effect";

export const ScoreChanged = Event.make("ScoreChanged", {
	payload: { score: Schema.Number },
});

export const Welcome = Event.make("Welcome");

export const Join = Action.make("Join");

export const SetScore = Action.make("SetScore", {
	payload: { score: Schema.Number },
});

export const Game = Actor.make("Game", {
	actions: [Join, SetScore],
	events: [ScoreChanged, Welcome],
});

export const GameLive = Game.toLayer(
	Effect.gen(function* () {
		const events = yield* Game.events;

		return Game.of({
			// Sends `Welcome` only to the caller.
			Join: ({ connection }) =>
				Option.match(connection, {
					onNone: () => Effect.void,
					onSome: (caller) => events.send(caller, Welcome),
				}),
			// Sends `ScoreChanged` to every connection.
			SetScore: ({ payload }) =>
				events.broadcast(ScoreChanged, { score: payload.score }),
		});
	}),
);
