import { Actor } from "@rivetkit/effect";
import { Effect } from "effect";
import { HttpServerRequest, HttpServerResponse } from "effect/http";
import { Socket } from "effect/socket";

export const Gateway = Actor.make("Gateway");

export const GatewayLive = Gateway.toLayer(
	Gateway.of(
		{},
		{
			// Answers each request with its method and URL as JSON.
			http: Effect.gen(function* () {
				const request = yield* HttpServerRequest.HttpServerRequest;
				return yield* HttpServerResponse.json({
					method: request.method,
					url: request.url,
				});
			}),
			// Sends each text message back until the client disconnects.
			webSocket: Effect.gen(function* () {
				const socket = yield* Socket.Socket;
				const { write } = yield* socket.writer;
				const pull = yield* Socket.readerString(socket);
				yield* pull.pipe(
					Effect.flatMap(Effect.forEach(write)),
					Effect.forever,
				);
			}),
		},
	),
);
