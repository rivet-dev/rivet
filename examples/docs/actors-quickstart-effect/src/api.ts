import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
});

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
});

export const Counter = Actor.make("Counter", {
	actions: [Increment, GetCount],
});
