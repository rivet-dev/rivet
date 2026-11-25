import { Action, Actor, Event } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const ScoreChanged = Event.make("ScoreChanged", {
	payload: { score: Schema.Number },
});

export const SetScore = Action.make("SetScore", {
	payload: { score: Schema.Number },
});

export const Match = Actor.make("Match", {
	actions: [SetScore],
	events: [ScoreChanged],
	connection: {
		params: Schema.Struct({
			role: Schema.Literals(["player", "spectator"]),
		}),
	},
});

export const MatchLive = Match.toLayer(
	Effect.gen(function* () {
		const events = yield* Match.events;

		return Match.of(
			{
				SetScore: ({ payload }) =>
					events.broadcast(ScoreChanged, { score: payload.score }),
			},
			{
				authorize: () => Effect.void,
				canSubscribe: {
					// Only players get the score.
					ScoreChanged: ({ params }) =>
						Effect.succeed(params.role === "player"),
				},
			},
		);
	}),
);
