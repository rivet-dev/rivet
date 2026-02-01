import type { WSContext } from "hono/ws";
import type { Unsubscribe } from "nanoevents";
import type { UpgradeWebSocketArgs } from "@/actor/router-websocket-endpoints";
import type { AnyActorInstance, RivetMessageEvent } from "@/mod";
import type { ToClient } from "@/schemas/actor-inspector/mod";
import {
	CURRENT_VERSION as INSPECTOR_CURRENT_VERSION,
	TO_CLIENT_VERSIONED as toClient,
	TO_SERVER_VERSIONED as toServer,
} from "@/schemas/actor-inspector/versioned";
import { assertUnreachable } from "@/utils";
import { inspectorLogger } from "./log";

export async function handleWebSocketInspectorConnect({
	actor,
}: {
	actor: AnyActorInstance;
}): Promise<UpgradeWebSocketArgs> {
	const inspector = actor.inspector;

	const listeners: Unsubscribe[] = [];
	return {
		// NOTE: onOpen cannot be async since this messes up the open event listener order
		onOpen: (_evt: any, ws: WSContext) => {
			sendMessage(ws, {
				body: {
					tag: "Init",
					val: {
						connections: inspector.getConnections(),
						events: inspector.getLastEvents(),
						rpcs: inspector.getRpcs(),
						state: inspector.isStateEnabled()
							? inspector.getState()
							: null,
						isStateEnabled: inspector.isStateEnabled(),
						isDatabaseEnabled: inspector.isDatabaseEnabled(),
						queueSize: BigInt(inspector.getQueueSize()),
					},
				},
			});

			listeners.push(
				inspector.emitter.on("stateUpdated", () => {
					sendMessage(ws, {
						body: {
							tag: "StateUpdated",
							val: { state: inspector.getState() },
						},
					});
				}),
				inspector.emitter.on("connectionsUpdated", () => {
					sendMessage(ws, {
						body: {
							tag: "ConnectionsUpdated",
							val: { connections: inspector.getConnections() },
						},
					});
				}),
				inspector.emitter.on("eventFired", () => {
					sendMessage(ws, {
						body: {
							tag: "EventsUpdated",
							val: { events: inspector.getLastEvents() },
						},
					});
				}),
				inspector.emitter.on("eventsChanged", () => {
					sendMessage(ws, {
						body: {
							tag: "EventsUpdated",
							val: { events: inspector.getLastEvents() },
						},
					});
				}),
				inspector.emitter.on("queueUpdated", () => {
					sendMessage(ws, {
						body: {
							tag: "QueueUpdated",
							val: {
								queueSize: BigInt(inspector.getQueueSize()),
							},
						},
					});
				}),
			);
		},
		onMessage: async (evt: RivetMessageEvent, ws: WSContext) => {
			try {
				const message = receiveMessage(evt.data);

				if (message.body.tag === "PatchStateRequest") {
					const { state } = message.body.val;
					inspector.setState(state);
					return;
				} else if (message.body.tag === "ActionRequest") {
					const { name, args, id } = message.body.val;
					const result = await inspector.executeAction(name, args);
					sendMessage(ws, {
						body: {
							tag: "ActionResponse",
							val: {
								rid: id,
								output: result,
							},
						},
					});
				} else if (message.body.tag === "StateRequest") {
					sendMessage(ws, {
						body: {
							tag: "StateResponse",
							val: {
								rid: message.body.val.id,
								state: inspector.isStateEnabled()
									? inspector.getState()
									: null,
								isStateEnabled: inspector.isStateEnabled(),
							},
						},
					});
				} else if (message.body.tag === "ConnectionsRequest") {
					sendMessage(ws, {
						body: {
							tag: "ConnectionsResponse",
							val: {
								rid: message.body.val.id,
								connections: inspector.getConnections(),
							},
						},
					});
				} else if (message.body.tag === "EventsRequest") {
					sendMessage(ws, {
						body: {
							tag: "EventsResponse",
							val: {
								rid: message.body.val.id,
								events: inspector.getLastEvents(),
							},
						},
					});
				} else if (message.body.tag === "ClearEventsRequest") {
					inspector.clearEvents();
					sendMessage(ws, {
						body: {
							tag: "EventsResponse",
							val: {
								rid: message.body.val.id,
								events: [],
							},
						},
					});
				} else if (message.body.tag === "RpcsListRequest") {
					sendMessage(ws, {
						body: {
							tag: "RpcsListResponse",
							val: {
								rid: message.body.val.id,
								rpcs: inspector.getRpcs(),
							},
						},
					});
				} else {
					assertUnreachable(message.body);
				}
			} catch (error) {
				inspectorLogger().warn(
					{ error },
					"Failed to handle inspector WS message",
				);
			}
		},
		onClose: (
			_event: {
				wasClean: boolean;
				code: number;
				reason: string;
			},
			_ws: WSContext,
		) => {
			for (const unsubscribe of listeners) {
				unsubscribe();
			}
		},
		onError: (_error: unknown) => {
			inspectorLogger().warn(
				{ error: _error },
				"WebSocket inspector connection error",
			);
		},
	};
}

function sendMessage(ws: WSContext, message: ToClient) {
	ws.send(
		toClient.serializeWithEmbeddedVersion(
			message,
			INSPECTOR_CURRENT_VERSION,
		) as unknown as ArrayBuffer,
	);
}

function receiveMessage(data: ArrayBuffer) {
	return toServer.deserializeWithEmbeddedVersion(new Uint8Array(data));
}
