import type { HibernatingWebSocketMetadata } from "@rivetkit/engine-runner";
import onChange from "@rivetkit/on-change";
import * as cbor from "cbor-x";
import invariant from "invariant";
import { isCborSerializable } from "@/common/utils";
import * as errors from "../errors";
import { assertUnreachable } from "../utils";
import { CONN_ACTOR_SYMBOL, type Conn } from "./mod";
import type { PersistedConn } from "./persisted";

/** Pick a subset of persisted data used to represent ephemeral connections */
export type EphemeralConn<CP, CS> = Pick<
	PersistedConn<CP, CS>,
	"id" | "parameters" | "state"
>;

export type ConnDataInput<CP, CS> =
	| { ephemeral: EphemeralConn<CP, CS> }
	| { hibernatable: PersistedConn<CP, CS> };

export type ConnData<CP, CS> =
	| {
			ephemeral: {
				/** In-memory data representing this connection */
				data: EphemeralConn<CP, CS>;
			};
	  }
	| {
			hibernatable: {
				/** Persisted data with on-change proxy */
				data: PersistedConn<CP, CS>;
				/** Raw persisted data without proxy */
				dataRaw: PersistedConn<CP, CS>;
			};
	  };

/**
 * Manages connection state persistence, proxying, and change tracking.
 * Handles automatic state change detection for connection-specific state.
 */
export class StateManager<CP, CS> {
	#conn: Conn<any, CP, CS, any, any, any>;

	/**
	 * Data representing this connection.
	 *
	 * This is stored as a struct for both ephemeral and hibernatable conns in
	 * order to keep the separation clear between the two.
	 */
	#data!: ConnData<CP, CS>;

	constructor(
		conn: Conn<any, CP, CS, any, any, any>,
		data: ConnDataInput<CP, CS>,
	) {
		this.#conn = conn;

		if ("ephemeral" in data) {
			this.#data = { ephemeral: { data: data.ephemeral } };
		} else if ("hibernatable" in data) {
			// Listen for changes to the object
			const persistRaw = data.hibernatable;
			const persist = onChange(
				persistRaw,
				(
					path: string,
					value: any,
					_previousValue: any,
					_applyData: any,
				) => {
					this.#handleChange(path, value);
				},
				{ ignoreDetached: true },
			);
			this.#data = {
				hibernatable: { data: persist, dataRaw: persistRaw },
			};
		} else {
			assertUnreachable(data);
		}
	}

	/**
	 * Returns the ephemeral or persisted data for this connectioned.
	 *
	 * This property is used to be able to treat both memory & persist conns
	 * identical by looking up the correct underlying data structure.
	 */
	get ephemeralData(): EphemeralConn<CP, CS> {
		if ("hibernatable" in this.#data) {
			return this.#data.hibernatable.data;
		} else if ("ephemeral" in this.#data) {
			return this.#data.ephemeral.data;
		} else {
			return assertUnreachable(this.#data);
		}
	}

	get hibernatableData(): PersistedConn<CP, CS> | undefined {
		if ("hibernatable" in this.#data) {
			return this.#data.hibernatable.data;
		} else {
			return undefined;
		}
	}

	hibernatableDataOrError(): PersistedConn<CP, CS> {
		const hibernatable = this.hibernatableData;
		invariant(hibernatable, "missing hibernatable data");
		return hibernatable;
	}

	get hibernatableDataRaw(): PersistedConn<CP, CS> | undefined {
		if ("hibernatable" in this.#data) {
			return this.#data.hibernatable.dataRaw;
		} else {
			return undefined;
		}
	}

	get stateEnabled(): boolean {
		return this.#conn[CONN_ACTOR_SYMBOL].connStateEnabled;
	}

	get state(): CS {
		this.#validateStateEnabled();
		const state = this.ephemeralData.state;
		if (!state) throw new Error("state should exists");
		return state;
	}

	set state(value: CS) {
		this.#validateStateEnabled();
		this.ephemeralData.state = value;
	}

	#validateStateEnabled() {
		if (!this.#conn[CONN_ACTOR_SYMBOL].connStateEnabled) {
			throw new errors.ConnStateNotEnabled();
		}
	}

	#handleChange(path: string, value: any) {
		// NOTE: This will only be called for hibernatable conns since only
		// hibernatable conns have the on-change proxy

		// Validate CBOR serializability for state changes
		if (path.startsWith("state")) {
			let invalidPath = "";
			if (
				!isCborSerializable(
					value,
					(invalidPathPart: string) => {
						invalidPath = invalidPathPart;
					},
					"",
				)
			) {
				throw new errors.InvalidStateType({
					path: path + (invalidPath ? `.${invalidPath}` : ""),
				});
			}
		}

		this.#conn[CONN_ACTOR_SYMBOL].rLog.debug({
			msg: "conn onChange triggered",
			connId: this.#conn.id,
			path,
		});

		// Notify actor that this connection has changed
		this.#conn[
			CONN_ACTOR_SYMBOL
		].connectionManager.markConnWithPersistChanged(this.#conn);
	}

	addSubscription({ eventName }: { eventName: string }) {
		const hibernatable = this.hibernatableData;
		if (!hibernatable) return;
		hibernatable.subscriptions.push({
			eventName,
		});
	}

	removeSubscription({ eventName }: { eventName: string }) {
		const hibernatable = this.hibernatableData;
		if (!hibernatable) return;
		const subIdx = hibernatable.subscriptions.findIndex(
			(s) => s.eventName === eventName,
		);
		if (subIdx !== -1) {
			hibernatable.subscriptions.splice(subIdx, 1);
		}
		return subIdx !== -1;
	}
}
