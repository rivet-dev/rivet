import { Effect } from "effect";
import { Socket } from "effect/socket";
import { Counter } from "./api";

export const program = Effect.gen(function* () {
	const counter = (yield* Counter.client).getOrCreate("my-counter");

	// Raw HTTP request
	const response = yield* counter.fetch("/history");
	yield* Effect.log(yield* Effect.promise(() => response.json()));

	// Raw WebSocket connection
	const socket = yield* counter.webSocket("/stream");
	const { write } = yield* socket.writer;
	const pull = yield* Socket.readerString(socket);
	yield* write("hello");
	yield* Effect.log(yield* pull);
}).pipe(Effect.scoped);
