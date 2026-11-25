import { Action, Actor } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const Notify = Action.make("Notify", {
	payload: { text: Schema.String },
});

export const Remind = Action.make("Remind", {
	payload: { text: Schema.String },
	success: Schema.String,
});

export const Reminder = Actor.make("Reminder", {
	actions: [Remind, Notify],
});

export const ReminderLive = Reminder.toLayer(
	Effect.gen(function* () {
		const schedule = yield* Reminder.schedule;

		return Reminder.of({
			// Calls `Notify` in 10 minutes and returns the schedule id.
			Remind: ({ payload }) =>
				schedule
					.after("10 minutes", Notify, { text: payload.text })
					.pipe(Effect.orDie),
			Notify: ({ payload }) => Effect.log(payload.text),
		});
	}),
);
