import { Action, Actor } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const Ping = Action.make("Ping", {
	payload: { url: Schema.String },
	success: Schema.Number,
});

export const Probe = Actor.make("Probe", {
	actions: [Ping],
});

export const ProbeLive = Probe.toLayer(({ rawRivetkitContext }) =>
	Probe.of({
		// Aborts the request when the actor shuts down.
		Ping: ({ payload }) =>
			Effect.promise(() =>
				fetch(payload.url, { signal: rawRivetkitContext.abortSignal }),
			).pipe(Effect.map((response) => response.status)),
	}),
);
