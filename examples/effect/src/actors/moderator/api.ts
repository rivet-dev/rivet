import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export class BannedWordsError extends Schema.TaggedErrorClass<BannedWordsError>()(
	"BannedWordsError",
	{
		message: Schema.String,
	},
) {}

export const Review = Action.make("Review", {
	payload: { text: Schema.String },
	error: BannedWordsError,
});

export const Stats = Action.make("Stats", {
	success: Schema.Struct({
		reviewed: Schema.Number,
	}),
});

export const Moderator = Actor.make("moderator", {
	actions: [Review, Stats],
});
