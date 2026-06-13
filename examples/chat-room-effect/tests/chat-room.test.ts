import { assert, layer } from "@effect/vitest";
import { Registry } from "@rivetkit/effect";
import { Effect, Layer, Random } from "effect";
import { ChatRoom, MemberNotInRoomError } from "../src/actors/chat-room/api.ts";
import { ChatRoomLive, RoomPolicyLive } from "../src/actors/chat-room/live.ts";
import { BannedWordsError } from "../src/actors/moderator/api.ts";
import { ModeratorLive } from "../src/actors/moderator/live.ts";

// `Registry.test` boots the actors in-process against a local engine. With no
// endpoint configured on `Registry.layer`, it auto-spawns a `rivet-engine` for
// the duration of the suite, the same way `setupTest` does for the other
// examples. It also provides `Client`, so `ChatRoom.client` resolves here.
const TestLayer = Registry.test.pipe(
	Layer.provideMerge(
		Layer.mergeAll(
			ModeratorLive,
			ChatRoomLive.pipe(Layer.provide(RoomPolicyLive)),
		),
	),
	Layer.provide(Registry.layer()),
);

// A fresh room key per test keeps actor state from bleeding across cases.
const freshRoom = Effect.gen(function* () {
	const client = yield* ChatRoom.client;
	return client.getOrCreate(`chatroom_${yield* Random.nextUUIDv4}`);
});

layer(TestLayer)("chat-room-effect", (it) => {
	it.effect("joins a room and reads message history", () =>
		Effect.gen(function* () {
			const room = yield* freshRoom;
			yield* room.Initialize({ name: "Effect Lovers" });

			// The room seeds an "Admin" member, so Alice is the second.
			const { memberCount } = yield* room.Join({ name: "Alice" });
			assert.strictEqual(memberCount, 2);

			yield* room.SendMessage({
				sender: "Alice",
				text: "hello from Effect",
			});

			const history = yield* room.GetHistory();
			assert.strictEqual(history.length, 1);
			assert.strictEqual(history[0].sender, "Alice");
			assert.strictEqual(history[0].text, "hello from Effect");
		}),
	);

	it.effect("rejects messages from non-members", () =>
		Effect.gen(function* () {
			const room = yield* freshRoom;
			yield* room.Initialize({ name: "Closed Room" });

			const exit = yield* room
				.SendMessage({ sender: "Mallory", text: "let me in" })
				.pipe(Effect.flip, Effect.exit);

			assert.isTrue(exit._tag === "Success");
			if (exit._tag === "Success") {
				assert.instanceOf(exit.value, MemberNotInRoomError);
			}
		}),
	);

	it.effect("rejects banned words through the moderator actor", () =>
		Effect.gen(function* () {
			const room = yield* freshRoom;
			yield* room.Initialize({ name: "Moderated Room" });
			yield* room.Join({ name: "Alice" });

			// The error originates in the Moderator actor and flows back
			// through SendMessage's declared error channel.
			const exit = yield* room
				.SendMessage({ sender: "Alice", text: "this contains spam" })
				.pipe(Effect.flip, Effect.exit);

			assert.isTrue(exit._tag === "Success");
			if (exit._tag === "Success") {
				assert.instanceOf(exit.value, BannedWordsError);
			}
		}),
	);
});
