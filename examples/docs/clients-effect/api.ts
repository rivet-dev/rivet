import { Action, Actor, Event } from "@rivetkit/effect";
import { Schema } from "effect";

export class CounterOverflow extends Schema.TaggedError<CounterOverflow>()(
	"CounterOverflow",
	{ limit: Schema.Number },
) {}

export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
	error: CounterOverflow,
});

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
});

export const CountChanged = Event.make("CountChanged", {
	payload: { count: Schema.Number },
});

export const Counter = Actor.make("Counter", {
	actions: [Increment, GetCount],
	events: [CountChanged],
});
