import onChange from "on-change";
import { isCborSerializable } from "@/common/utils";
import * as errors from "../errors";
import type { PersistedConn } from "../instance/persisted";
import { CONN_ACTOR_SYMBOL, CONN_STATE_ENABLED_SYMBOL, type Conn } from "./mod";

/**
 * Manages connection state persistence, proxying, and change tracking.
 * Handles automatic state change detection for connection-specific state.
 */
export class StateManager<CP, CS> {
	#conn: Conn<any, CP, CS, any, any, any>;

	// State tracking
	#persist!: PersistedConn<CP, CS>;
	#persistRaw!: PersistedConn<CP, CS>;
	#changed = false;

	constructor(conn: Conn<any, CP, CS, any, any, any>) {
		this.#conn = conn;
	}

	// MARK: - Public API

	get persist(): PersistedConn<CP, CS> {
		return this.#persist;
	}

	get persistRaw(): PersistedConn<CP, CS> {
		return this.#persistRaw;
	}

	get changed(): boolean {
		return this.#changed;
	}

	get stateEnabled(): boolean {
		return this.#conn[CONN_ACTOR_SYMBOL].connStateEnabled;
	}

	get state(): CS {
		this.#validateStateEnabled();
		if (!this.#persist.state) throw new Error("state should exists");
		return this.#persist.state;
	}

	set state(value: CS) {
		this.#validateStateEnabled();
		this.#persist.state = value;
	}

	get params(): CP {
		return this.#persist.params;
	}

	// MARK: - Initialization

	/**
	 * Creates proxy for persist object that handles automatic state change detection.
	 */
	initPersistProxy(target: PersistedConn<CP, CS>) {
		// Set raw persist object
		this.#persistRaw = target;

		// If this can't be proxied, return raw value
		if (target === null || typeof target !== "object") {
			this.#persist = target;
			return;
		}

		// Listen for changes to the object
		this.#persist = onChange(
			target,
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
	}

	// MARK: - Change Management

	/**
	 * Returns whether this connection has unsaved changes
	 */
	hasChanges(): boolean {
		return this.#changed;
	}

	/**
	 * Marks changes as saved
	 */
	markSaved() {
		this.#changed = false;
	}

	// MARK: - Private Helpers

	#validateStateEnabled() {
		if (!this.stateEnabled) {
			throw new errors.ConnStateNotEnabled();
		}
	}

	#handleChange(path: string, value: any) {
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

		this.#changed = true;
		this.#conn[CONN_ACTOR_SYMBOL].rLog.debug({
			msg: "conn onChange triggered",
			connId: this.#conn.id,
			path,
		});

		// Notify actor that this connection has changed
		this.#conn[CONN_ACTOR_SYMBOL].connectionManager.markConnChanged(
			this.#conn,
		);
	}
}
