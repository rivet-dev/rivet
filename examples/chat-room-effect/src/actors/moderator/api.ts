import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export class BannedWordsError extends Schema.TaggedError<BannedWordsError>()(
	"BannedWordsError",
	{
		message: Schema.String,
	},
) {}

export const Review = Action.make("Review", {
	payload: { text: Schema.String },
	error: BannedWordsError,
});

export const Moderator = Actor.make("Moderator", {
	actions: [Review],
});
