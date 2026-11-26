import { Effect } from "effect";
import { Room } from "./room";

export const program = Effect.gen(function* () {
	const room = (yield* Room.client).getOrCreate("general", {
		params: { authToken: "jwt-token-here" },
	});

	const history = yield* room
		.GetHistory()
		.pipe(Effect.catchTag("Unauthorized", () => Effect.succeed([])));
	yield* Effect.log(`${history.length} messages`);
});
