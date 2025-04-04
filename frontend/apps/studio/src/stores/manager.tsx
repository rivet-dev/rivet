import {
	ToClientSchema,
	type ToClient,
	type ToServer,
	type Actor as InspectorActor,
} from "actor-core/inspector/protocol/manager";
import { toast } from "@rivet-gg/components";
import { atom } from "jotai";
import { atomEffect } from "jotai-effect";
import {
	type Actor,
	actorBuildsAtom,
	ActorFeature,
	actorsAtom,
	createActorAtom,
} from "@rivet-gg/components/actors";
import { createClient } from "actor-core/client";

const createConnection = ({
	onMessage,
	onConnect,
	onDisconnect,
}: {
	onMessage?: (msg: ToClient) => void;
	onConnect?: () => void;
	onDisconnect?: () => void;
} = {}) => {
	const ws = new WebSocket(
		`ws://localhost:${ACTOR_CORE_MANAGER_PORT}/manager/inspect`,
	);

	const connectionTimeout = setTimeout(() => {
		if (ws.readyState !== WebSocket.OPEN) {
			ws.close();
		}
	}, 1500);

	ws.addEventListener("open", () => {
		onConnect?.();
		clearTimeout(connectionTimeout);
		ws.send(JSON.stringify({ type: "info" }));
	});

	ws.addEventListener("message", (event) => {
		const data = JSON.parse(event.data);
		const result = ToClientSchema.safeParse(data);
		if (!result.success) {
			console.error("Invalid data", result.error);
			return;
		}
		if (onMessage) {
			onMessage(result.data);
		}
	});

	ws.addEventListener("close", () => {
		onDisconnect?.();
	});

	ws.addEventListener("error", (event) => {
		console.error("WebSocket error", event);
		ws.close();
	});

	return ws;
};

export const ACTOR_CORE_MANAGER_PORT = 6420;

export const connectionStateAtom = atom<"disconnected" | "connected">(
	"disconnected",
);

export const websocketAtom = atom<WebSocket | null>(null);
export const initiallyConnectedAtom = atom(false);

export const connectionEffect = atomEffect((get, set) => {
	if (get.peek(websocketAtom)) {
		// effect already ran
		return;
	}

	let ws: WebSocket | null = null;
	let reconnectTimeout: ReturnType<typeof window.setTimeout> | undefined =
		undefined;

	function reconnect() {
		ws = createConnection({
			onConnect: () => {
				set(websocketAtom, ws);
				set(initiallyConnectedAtom, true);
				set(connectionStateAtom, "connected");

				toast.success("Connected to Rivet Studio", {
					id: "ws-reconnect",
				});
			},
			onDisconnect: () => {
				set(connectionStateAtom, "disconnected");

				if (get.peek(initiallyConnectedAtom)) {
					toast.loading("Reconnecting...", { id: "ws-reconnect" });
				}
				reconnectTimeout = setTimeout(() => {
					reconnect();
				}, 500);
			},
			onMessage: (msg) => {
				if (msg.type === "info") {
					set(initiallyConnectedAtom, true);
					set(actorsAtom, msg.actors.map(convertActor));
					set(
						actorBuildsAtom,
						msg.types.map((type) => ({
							id: type,
							name: type,
							tags: { current: "true" },
							contentLength: 0,
							createdAt: new Date(),
						})),
					);

					set(connectionStateAtom, "connected");

					const managerEndpoint = `http://localhost:${ACTOR_CORE_MANAGER_PORT}`;
					set(createActorAtom, {
						endpoint: managerEndpoint,
						isCreating: false,
						async create(values) {
							const client = createClient(managerEndpoint);

							return client.create(values.name, {
								params: values.params,
								create: {
									tags: values.tags,
									region: values.region,
								},
							});
						},
					});
				}
				if (msg.type === "actors") {
					const existingActors = get.peek(actorsAtom);
					set(
						actorsAtom,
						msg.actors.map((actor) => {
							const existingActor = existingActors.find(
								(a) => a.id === actor.id,
							);
							if (existingActor) {
								// remove logs from existing actor (bc logs are atom, and it will cause to recreate the atom)
								const { logs, ...rest } = existingActor;
								return {
									...existingActor,
									...rest,
								};
							}
							return convertActor(actor);
						}),
					);
				}
			},
		});
	}

	reconnect();

	return () => {
		if (ws) {
			clearTimeout(reconnectTimeout);
			ws.close();
			ws = null;
		}
	};
});

export const sendAtom = atom(null, (get, _set, msg: ToServer) => {
	const ws = get(websocketAtom);
	if (!ws) {
		console.error("WebSocket not connected");
		return;
	}
	ws.send(JSON.stringify(msg));
});

function convertActor(actor: InspectorActor): Actor {
	return {
		...actor,
		logs: atom({
			errors: { lines: [], timestamps: [], ids: [] },
			logs: { lines: [], timestamps: [], ids: [] },
		}),
		endpoint: `http://localhost:${ACTOR_CORE_MANAGER_PORT}/actors/${actor.id}`,
		status: "running",
		region: "local",
		network: null,
		runtime: null,
		resources: null,
		lifecycle: {},
		features: [
			ActorFeature.Console,
			ActorFeature.State,
			ActorFeature.Connections,
			ActorFeature.Config,
		],
	};
}
