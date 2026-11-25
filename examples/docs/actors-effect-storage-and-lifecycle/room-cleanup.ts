import { Action, Actor } from "@rivetkit/effect";
import { Effect } from "effect";

// Stand-in for a client of another system.
const openUpstream = (roomId: string) =>
	Effect.succeed({
		send: (text: string) => Effect.log(`${roomId}: ${text}`),
		close: Effect.log(`Closed the connection of ${roomId}`),
		deleteRoom: Effect.log(`Deleted ${roomId} upstream`),
	});

export const Send = Action.make("Send");

export const Room = Actor.make("Room", { actions: [Send] });

export const RoomLive = Room.toLayer(
	Effect.gen(function* () {
		const { actorId } = yield* Actor.CurrentAddress;
		const upstream = yield* openUpstream(actorId);

		// Runs on every sleep and destroy: this wake is over.
		yield* Effect.addFinalizer(() => upstream.close);

		return Room.of(
			{ Send: () => upstream.send("hello") },
			{
				// Runs only when the awake actor is destroyed: the actor is gone.
				onDestroy: upstream.deleteRoom,
			},
		);
	}),
);
