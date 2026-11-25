import { Effect } from "effect";
import { Room } from "./room";

export const program = Effect.gen(function* () {
	const room = (yield* Room.client).getOrCreate("lobby", {
		params: { userId: "alice" },
	});

	// Fails with `Refused` if `authorize` refuses the connection.
	const userId = yield* room.WhoAmI();
	yield* Effect.log(`Connected as ${userId}`);
});
