import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export class DivisionByZero extends Schema.TaggedError<DivisionByZero>()(
	"DivisionByZero",
	{ dividend: Schema.Number },
) {}

export const Divide = Action.make("Divide", {
	payload: { dividend: Schema.Number, divisor: Schema.Number },
	success: Schema.Number,
	error: DivisionByZero,
});

export const Calculator = Actor.make("Calculator", {
	actions: [Divide],
});
