import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export class Unauthorized extends Schema.TaggedError<Unauthorized>()(
	"Unauthorized",
	{},
) {}

export const GetHistory = Action.make("GetHistory", {
	success: Schema.Array(Schema.String),
});

export const Room = Actor.make("Room", {
	actions: [GetHistory],
	connection: {
		params: Schema.Struct({ authToken: Schema.String }),
		error: Unauthorized,
	},
});
