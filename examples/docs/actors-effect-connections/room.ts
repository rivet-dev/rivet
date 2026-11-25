import { Action, Actor } from "@rivetkit/effect";
import { Effect, Option, Schema } from "effect";

export class Refused extends Schema.TaggedError<Refused>()("Refused", {
	userId: Schema.String,
}) {}

export const WhoAmI = Action.make("WhoAmI", { success: Schema.String });

export const Kick = Action.make("Kick", {
	payload: { userId: Schema.String },
});

export const Room = Actor.make("Room", {
	actions: [WhoAmI, Kick],
	connection: {
		params: Schema.Struct({ userId: Schema.String }),
		error: Refused,
	},
});

export const RoomLive = Room.toLayer(
	Effect.gen(function* () {
		const connections = yield* Room.connections;

		return Room.of(
			{
				WhoAmI: ({ connection }) =>
					Option.match(connection, {
						onNone: () => Effect.die("WhoAmI needs a connection"),
						onSome: ({ params }) => Effect.succeed(params.userId),
					}),
				// Closes each open connection of the user.
				Kick: Effect.fn(function* ({ payload }) {
					const open = yield* connections.list;
					yield* Effect.forEach(
						open.filter(
							({ params }) => params.userId === payload.userId,
						),
						(connection) =>
							connections.disconnect(connection, "kicked"),
						{ discard: true },
					).pipe(Effect.orDie);
				}),
			},
			{
				authorize: ({ userId }) =>
					userId === "mallory"
						? Effect.fail(new Refused({ userId }))
						: Effect.void,
				onConnect: ({ params }) =>
					Effect.log(`${params.userId} joined`),
				onDisconnect: ({ params }) =>
					Effect.log(`${params.userId} left`),
			},
		);
	}),
);
