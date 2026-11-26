import { Action, Actor, Client } from "@rivetkit/effect";
import { Effect, Layer, Schema } from "effect";

export class BannedWord extends Schema.TaggedError<BannedWord>()("BannedWord", {
	word: Schema.String,
}) {}

export const Review = Action.make("Review", {
	payload: { text: Schema.String },
	error: BannedWord,
});

export const Moderator = Actor.make("Moderator", {
	actions: [Review],
});

export const Post = Action.make("Post", {
	payload: { text: Schema.String },
	error: BannedWord,
});

export const Feed = Actor.make("Feed", {
	actions: [Post],
});

export const ModeratorLive = Moderator.toLayer(
	Moderator.of({
		Review: ({ payload }) =>
			payload.text.includes("spam")
				? Effect.fail(new BannedWord({ word: "spam" }))
				: Effect.void,
	}),
);

export const FeedLive = Feed.toLayer(
	Effect.gen(function* () {
		const moderators = yield* Moderator.client;

		return Feed.of({
			// `BannedWord` from `Review` reaches the caller of `Post`.
			Post: ({ payload }) =>
				moderators
					.getOrCreate("main")
					.Review({ text: payload.text })
					.pipe(Effect.catchTag("RivetError", Effect.die)),
		});
	}),
);

export const ActorsLayer = Layer.mergeAll(ModeratorLive, FeedLive).pipe(
	Layer.provide(Client.layer({ endpoint: "http://localhost:6420" })),
);
