import { NodeRuntime } from "@effect/platform-node";
import { Client } from "@rivetkit/effect";
import { Effect, Random } from "effect";
import {
	type BannedWordsError,
	ChatRoom,
	type MemberNotInRoomError,
} from "./actors/mod.ts";
import { PrettyLoggerLayer } from "./logger.ts";

const program = Effect.gen(function* () {
	// `Actor.client` yields a typed accessor backed by the Effect SDK client layer.
	const chatRoomClient = yield* ChatRoom.client;
	const room = chatRoomClient.getOrCreate(
		`chatroom_${yield* Random.nextUUIDv4}`,
	);

	yield* Effect.addFinalizer(
		Effect.fnUntraced(function* () {
			yield* room.Archive().pipe(Effect.orDie);
			yield* Effect.log("archived room");
		}),
	);

	const roomName = "Effect Lovers";
	yield* room.Initialize({ name: roomName });
	yield* Effect.log(`created room ${roomName}`);

	const { memberCount } = yield* room.Join({ name: "Alice" });
	yield* Effect.log(`Alice joined; members=${memberCount}`);

	yield* room.SendMessage({
		sender: "Alice",
		text: "hello from Effect",
	});
	yield* Effect.log("Alice sent a message");

	// Domain errors declared on the action schema are caught by tag.
	yield* room
		.SendMessage({
			sender: "Mallory",
			text: "I should not be able to post",
		})
		.pipe(
			Effect.catchTag("MemberNotInRoomError", (e: MemberNotInRoomError) =>
				Effect.logWarning(`rejected non-member message: ${e.message}`),
			),
		);

	// Errors from nested actor-to-actor RPCs can flow through the caller action.
	yield* room
		.SendMessage({
			sender: "Alice",
			text: "this contains spam",
		})
		.pipe(
			Effect.catchTag("BannedWordsError", (e: BannedWordsError) =>
				Effect.logWarning(`rejected banned message: ${e.message}`),
			),
		);

	// A welcome message is scheduled by Join and internally dispatched through SendMessage.
	yield* Effect.sleep("1500 millis");

	const history = yield* room.GetHistory();
	const transcript = history
		.map((message) => `  ${message.sender}: ${message.text}`)
		.join("\n");
	yield* Effect.log(`message history:\n${transcript}`);
}).pipe(Effect.scoped);

const ClientLayer = Client.layer({ endpoint: "http://127.0.0.1:6420" });

program
	.pipe(Effect.provide(ClientLayer), Effect.provide(PrettyLoggerLayer))
	.pipe(NodeRuntime.runMain);
