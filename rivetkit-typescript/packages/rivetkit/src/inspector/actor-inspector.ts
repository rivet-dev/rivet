import * as cbor from "cbor-x";
import { createNanoEvents } from "nanoevents";
import { createHttpDriver } from "@/actor/conn/drivers/http";
import {
	CONN_DRIVER_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
} from "@/actor/conn/mod";
import { ActionContext } from "@/actor/contexts/action";
import * as actorErrors from "@/actor/errors";
import type { AnyActorInstance } from "@/mod";
import type * as schema from "@/schemas/actor-inspector/mod";
import { assertUnreachable, bufferToArrayBuffer } from "@/utils";
import type { Event } from "./mod";

interface ActorInspectorEmitterEvents {
	stateUpdated: (state: unknown) => void;
	connectionsUpdated: () => void;
	eventFired: (event: EventDetails) => void;
	eventsChanged: () => void;
	queueUpdated: () => void;
}

export type Connection = Omit<schema.Connection, "details"> & {
	details: unknown;
};

type EventDetails =
	| {
			type: "action";
			name: string;
			args: unknown[];
			connId: string;
	  }
	| {
			type: "subscribe";
			eventName: string;
			connId: string;
	  }
	| {
			type: "unsubscribe";
			eventName: string;
			connId: string;
	  }
	| {
			type: "event";
			eventName: string;
			args: unknown[];
			connId: string;
	  }
	| { type: "broadcast"; eventName: string; args: unknown[] };

/**
 * Provides a unified interface for inspecting actor external and internal state.
 */
export class ActorInspector {
	public readonly emitter = createNanoEvents<ActorInspectorEmitterEvents>();

	#lastEvents: Event[] = [];
	#lastQueueSize = 0;

	constructor(private readonly actor: AnyActorInstance) {
		this.#lastQueueSize = actor.queueManager?.size ?? 0;
		this.emitter.on("eventFired", (event) => {
			const commonParams = {
				id: crypto.randomUUID(),
				timestamp: BigInt(Date.now()),
			};

			this.#lastEvents.push({
				...commonParams,
				...transformEvent(event),
			});

			// keep the last 100 events
			if (this.#lastEvents.length > 100) {
				this.#lastEvents = this.#lastEvents.slice(-100);
			}
		});
	}

	getLastEvents() {
		return this.#lastEvents;
	}

	clearEvents() {
		this.#lastEvents = [];
		this.emitter.emit("eventsChanged");
	}

	getQueueSize() {
		return this.#lastQueueSize;
	}

	updateQueueSize(size: number) {
		if (this.#lastQueueSize === size) {
			return;
		}
		this.#lastQueueSize = size;
		this.emitter.emit("queueUpdated");
	}

	// actor accessor methods

	isDatabaseEnabled() {
		try {
			return this.actor.db !== undefined;
		} catch {
			return false;
		}
	}

	isStateEnabled() {
		return this.actor.stateEnabled;
	}

	getState() {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		return bufferToArrayBuffer(
			cbor.encode(this.actor.stateManager.persistRaw.state),
		);
	}

	getRpcs() {
		return this.actor.actions;
	}

	getConnections() {
		return Array.from(
			this.actor.connectionManager.connections.entries(),
		).map(([id, conn]) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			return {
				type: conn[CONN_DRIVER_SYMBOL]?.type,
				id,
				details: bufferToArrayBuffer(
					cbor.encode({
						type: conn[CONN_DRIVER_SYMBOL]?.type,
						params: conn.params as any,
						stateEnabled: connStateManager.stateEnabled,
						state: connStateManager.stateEnabled
							? connStateManager.state
							: undefined,
						subscriptions: conn.subscriptions.size,
						isHibernatable: conn.isHibernatable,
						// TODO: Include underlying hibernatable metadata +
						// path + headers
					}),
				),
			};
		});
	}
	async setState(state: ArrayBuffer) {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		this.actor.stateManager.state = cbor.decode(Buffer.from(state));
		await this.actor.stateManager.saveState({ immediate: true });
	}

	async executeAction(name: string, params: ArrayBuffer) {
		const conn = await this.actor.connectionManager.prepareAndConnectConn(
			createHttpDriver(),
			// TODO: This may cause issues
			undefined,
			undefined,
			undefined,
			undefined,
		);

		try {
			return bufferToArrayBuffer(
				cbor.encode(
					await this.actor.executeAction(
						new ActionContext(this.actor, conn),
						name,
						cbor.decode(Buffer.from(params)),
					),
				),
			);
		} finally {
			conn.disconnect();
		}
	}
}

function transformEvent(event: EventDetails) {
	if (event.type === "action") {
		return {
			body: {
				tag: "ActionEvent",
				val: {
					name: event.name,
					args: bufferToArrayBuffer(cbor.encode(event.args)),

					connId: event.connId,
				},
			},
		} as const;
	} else if (event.type === "subscribe") {
		return {
			body: {
				tag: "SubscribeEvent",
				val: {
					eventName: event.eventName,
					connId: event.connId,
				},
			},
		} as const;
	} else if (event.type === "unsubscribe") {
		return {
			body: {
				tag: "UnSubscribeEvent",
				val: {
					eventName: event.eventName,
					connId: event.connId,
				},
			},
		} as const;
	} else if (event.type === "event") {
		return {
			body: {
				tag: "FiredEvent",
				val: {
					eventName: event.eventName,
					args: bufferToArrayBuffer(cbor.encode(event.args)),
					connId: event.connId,
				},
			},
		} as const;
	} else if (event.type === "broadcast") {
		return {
			body: {
				tag: "BroadcastEvent",
				val: {
					eventName: event.eventName,
					args: bufferToArrayBuffer(cbor.encode(event.args)),
				},
			},
		} as const;
	} else {
		assertUnreachable(event);
	}
}
