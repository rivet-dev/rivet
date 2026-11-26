import { Action, Actor } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const Start = Action.make("Start");

export const SendDigest = Action.make("SendDigest", {
	payload: { channel: Schema.String },
});

export const Digest = Actor.make("Digest", {
	actions: [Start, SendDigest],
});

export const DigestLive = Digest.toLayer(
	Effect.gen(function* () {
		const cron = yield* Digest.cron;

		return Digest.of({
			// Calls `SendDigest` at 09:00 in Berlin, Monday to Friday.
			Start: () =>
				cron
					.set(
						{
							name: "weekday-digest",
							expression: "0 9 * * 1-5",
							timezone: "Europe/Berlin",
						},
						SendDigest,
						{ channel: "general" },
					)
					.pipe(Effect.orDie),
			SendDigest: ({ payload }) =>
				Effect.log(`Digest for ${payload.channel}`),
		});
	}),
);
