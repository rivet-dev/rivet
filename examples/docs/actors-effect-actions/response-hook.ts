import { Action, Actor } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const GetProfile = Action.make("GetProfile", {
	success: Schema.Struct({
		name: Schema.String,
		email: Schema.optional(Schema.String),
	}),
});

export const Profile = Actor.make("Profile", {
	actions: [GetProfile],
});

export const ProfileLive = Profile.toLayer(
	Profile.of(
		{
			GetProfile: () =>
				Effect.succeed({ name: "Ada", email: "ada@example.com" }),
		},
		{
			onBeforeActionResponse: {
				GetProfile: ({ success }) =>
					Effect.succeed({ name: success.name }),
			},
		},
	),
);
