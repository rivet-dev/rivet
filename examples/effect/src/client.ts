import { Effect, Random } from "effect";
import { Client } from "@rivetkit/effect";
import { Counter /*, IncrementBy */ } from "./actors/counter/api.ts";
import { ChatRoom } from "./actors/chat-room/api.ts";
import { Directory } from "./actors/directory/api.ts";
import { Moderator } from "./actors/moderator/api.ts";

const program = Effect.gen(function* () {
	const runId = yield* Random.nextUUIDv4;
	const counterClient = yield* Counter.client;
	const counter = counterClient.getOrCreate([`counter-effect-${runId}`]);

	const count = yield* counter.Increment({ amount: 5 });
	yield* Effect.log(`Increment(5) -> ${count}`);

	const total = yield* counter.GetCount();
	yield* Effect.log(`GetCount -> ${total}`);

	const chatRoomClient = yield* ChatRoom.client;
	const directoryClient = yield* Directory.client;
	const moderatorClient = yield* Moderator.client;

	const roomName = `effect-room-${runId}`;
	const room = chatRoomClient.getOrCreate([roomName]);
	const directory = directoryClient.getOrCreate(["main"]);
	const moderator = moderatorClient.getOrCreate(["main"]);

	yield* room.Initialize({ name: roomName });
	yield* Effect.log(`ChatRoom.Initialize`);

	const member = yield* room.Join({ name: "Alice" });
	yield* Effect.log(`ChatRoom.Join -> ${member.name}`);

	const sent = yield* room.SendMessage({
		sender: "Alice",
		text: "hello from Effect",
	});
	yield* Effect.log(`ChatRoom.SendMessage -> ok=${sent.ok}`);

	const rejected = yield* room.SendMessage({
		sender: "Alice",
		text: "this contains spam",
	});
	yield* Effect.log(
		`ChatRoom.SendMessage rejected -> ok=${rejected.ok} reason=${rejected.reason}`,
	);

	const history = yield* room.GetHistory();
	yield* Effect.log(`ChatRoom.GetHistory -> ${history.length} messages`);

	const members = yield* room.GetMembers();
	yield* Effect.log(`ChatRoom.GetMembers -> ${members.length} members`);

	const rooms = yield* directory.ListRooms();
	yield* Effect.log(`Directory.ListRooms -> ${rooms.length} rooms`);

	const stats = yield* moderator.Stats();
	yield* Effect.log(`Moderator.Stats -> reviewed=${stats.reviewed}`);

	// const newCount = yield* counter.send(IncrementBy({ amount: 3 }))
	// yield* Effect.log(`IncrementBy(3) -> ${newCount}`)
	//
	// // subscribe returns a Stream typed from the event schema.
	// yield* counter.subscribe("countChanged").pipe(
	// 	Stream.take(3),
	// 	Stream.runForEach((n) => Effect.log(`countChanged: ${n}`)),
	// )

	// Trigger overflow (limit: 20). The typed CounterOverflowError
	// round-trips through a UserError on the wire and decodes back
	// into the original error class — caught by the outer
	// `catchTag("CounterOverflowError", ...)`.
	const overflowed = yield* counter.Increment({ amount: 100 });
	yield* Effect.log(`Increment(100) [unexpected success]: ${overflowed}`);
}).pipe(
	Effect.catchTag("CounterOverflowError", (e) =>
		Effect.logError(
			`CounterOverflowError caught: limit=${e.limit} message="${e.message}"`,
		),
	),
);

const ClientLayer = Client.layer({ endpoint: "http://127.0.0.1:6420" });

program.pipe(Effect.provide(ClientLayer), Effect.runPromise).catch((err) => {
	console.error("client failed:", err);
	process.exit(1);
});
