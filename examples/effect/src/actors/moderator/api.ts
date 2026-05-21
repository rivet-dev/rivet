import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export class BannerWordsError extends Schema.TaggedErrorClass<BannerWordsError>()(
	"BannerWordsError",
	{
		message: Schema.String,
	},
) {}

export const Review = Action.make("Review", {
	payload: { text: Schema.String },
	error: BannerWordsError,
});

export const Stats = Action.make("Stats", {
	success: Schema.Struct({
		reviewed: Schema.Number,
	}),
});

export const Moderator = Actor.make("moderator", {
	actions: [Review, Stats],
});
