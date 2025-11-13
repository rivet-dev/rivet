import * as cbor from "cbor-x";
import onChange from "on-change";
import { isCborSerializable, stringifyError } from "@/common/utils";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import {
	ACTOR_VERSIONED,
	CONN_VERSIONED,
} from "@/schemas/actor-persist/versioned";
import {
	bufferToArrayBuffer,
	promiseWithResolvers,
	SinglePromiseQueue,
} from "@/utils";
import { type AnyConn, CONN_STATE_MANAGER_SYMBOL, Conn } from "../conn/mod";
import { convertConnToBarePersistedConn } from "../conn/persisted";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { isConnStatePath, isStatePath } from "../utils";
import { KEYS, makeConnKey } from "./kv";
import type { ActorInstance } from "./mod";
import { convertActorToBarePersisted, type PersistedActor } from "./persisted";

export interface SaveStateOptions {
	/**
	 * Forces the state to be saved immediately. This function will return when the state has saved successfully.
	 */
	immediate?: boolean;
	/** Bypass ready check for stopping. */
	allowStoppingState?: boolean;
	/**
	 * Maximum time in milliseconds to wait before forcing a save.
	 *
	 * If a save is already scheduled to occur later than this deadline, it will be rescheduled earlier.
	 */
	maxWait?: number;
}

/**
 * Manages actor state persistence, proxying, and synchronization.
 * Handles automatic state change detection and throttled persistence to KV storage.
 */
export class StateManager<S, CP, CS, I> {
	#actor: ActorInstance<S, CP, CS, any, I, any>;
	#actorDriver: ActorDriver;

	// State tracking
	#persist!: PersistedActor<S, I>;
	#persistRaw!: PersistedActor<S, I>;
	#persistChanged = false;
	#isInOnStateChange = false;

	// Save management
	#persistWriteQueue = new SinglePromiseQueue();
	#lastSaveTime = 0;
	#pendingSaveTimeout?: NodeJS.Timeout;
	#pendingSaveScheduledTimestamp?: number;
	#onPersistSavedPromise?: ReturnType<typeof promiseWithResolvers<void>>;

	// Configuration
	#config: any; // ActorConfig type
	#stateSaveInterval: number;

	constructor(
		actor: ActorInstance<S, CP, CS, any, I, any>,
		actorDriver: ActorDriver,
		config: any,
	) {
		this.#actor = actor;
		this.#actorDriver = actorDriver;
		this.#config = config;
		this.#stateSaveInterval = config.options.stateSaveInterval || 100;
	}

	// MARK: - Public API

	get persist(): PersistedActor<S, I> {
		return this.#persist;
	}

	get persistRaw(): PersistedActor<S, I> {
		return this.#persistRaw;
	}

	get persistChanged(): boolean {
		return this.#persistChanged;
	}

	get state(): S {
		this.#validateStateEnabled();
		return this.#persist.state;
	}

	set state(value: S) {
		this.#validateStateEnabled();
		this.#persist.state = value;
	}

	get stateEnabled(): boolean {
		return "createState" in this.#config || "state" in this.#config;
	}

	// MARK: - Initialization

	/**
	 * Initializes state from persisted data or creates new state.
	 */
	async initializeState(persistData: PersistedActor<S, I>): Promise<void> {
		if (!persistData.hasInitialized) {
			// Create initial state
			let stateData: unknown;
			if (this.stateEnabled) {
				this.#actor.rLog.info({ msg: "actor state initializing" });

				if ("createState" in this.#config) {
					stateData = await this.#config.createState(
						this.#actor.actorContext,
						persistData.input!,
					);
				} else if ("state" in this.#config) {
					stateData = structuredClone(this.#config.state);
				} else {
					throw new Error(
						"Both 'createState' or 'state' were not defined",
					);
				}
			} else {
				this.#actor.rLog.debug({ msg: "state not enabled" });
			}

			// Update persisted data
			persistData.state = stateData as S;
			persistData.hasInitialized = true;

			// Save initial state
			//
			// We don't use #savePersistInner because the actor is not fully
			// initialized yet
			const bareData = convertActorToBarePersisted<S, I>(persistData);
			await this.#actorDriver.kvBatchPut(this.#actor.id, [
				[
					KEYS.PERSIST_DATA,
					ACTOR_VERSIONED.serializeWithEmbeddedVersion(bareData),
				],
			]);
		}

		// Initialize proxy
		this.initPersistProxy(persistData);
	}

	/**
	 * Creates proxy for persist object that handles automatic state change detection.
	 */
	initPersistProxy(target: PersistedActor<S, I>) {
		// Set raw persist object
		this.#persistRaw = target;

		// Validate serializability
		if (target === null || typeof target !== "object") {
			let invalidPath = "";
			if (
				!isCborSerializable(
					target,
					(path) => {
						invalidPath = path;
					},
					"",
				)
			) {
				throw new errors.InvalidStateType({ path: invalidPath });
			}
			return target;
		}

		// Unsubscribe from old state
		if (this.#persist) {
			onChange.unsubscribe(this.#persist);
		}

		// Listen for changes to automatically write state
		this.#persist = onChange(
			target,
			(
				path: string,
				value: any,
				_previousValue: any,
				_applyData: any,
			) => {
				this.#handleStateChange(path, value);
			},
			{ ignoreDetached: true },
		);
	}

	// MARK: - State Persistence

	/**
	 * Forces the state to get saved.
	 */
	async saveState(opts: SaveStateOptions): Promise<void> {
		this.#actor.assertReady(opts.allowStoppingState);

		if (this.#persistChanged) {
			if (opts.immediate) {
				await this.#savePersistInner();
			} else {
				// Create promise for waiting
				if (!this.#onPersistSavedPromise) {
					this.#onPersistSavedPromise = promiseWithResolvers();
				}

				// Save throttled
				this.savePersistThrottled(opts.maxWait);

				// Wait for save
				await this.#onPersistSavedPromise?.promise;
			}
		}
	}

	/**
	 * Throttled save state method. Used to write to KV at a reasonable cadence.
	 *
	 * Passing a maxWait will override the stateSaveInterval with the min
	 * between that and the maxWait.
	 */
	savePersistThrottled(maxWait?: number) {
		const now = Date.now();
		const timeSinceLastSave = now - this.#lastSaveTime;

		// Calculate when the save should happen based on throttle interval
		let saveDelay = Math.max(
			0,
			this.#stateSaveInterval - timeSinceLastSave,
		);
		if (maxWait !== undefined) {
			saveDelay = Math.min(saveDelay, maxWait);
		}

		// Check if we need to reschedule the same timeout
		if (
			this.#pendingSaveTimeout !== undefined &&
			this.#pendingSaveScheduledTimestamp !== undefined
		) {
			// Check if we have an earlier save deadline
			const newScheduledTimestamp = now + saveDelay;
			if (newScheduledTimestamp < this.#pendingSaveScheduledTimestamp) {
				// Cancel existing timeout and reschedule
				clearTimeout(this.#pendingSaveTimeout);
				this.#pendingSaveTimeout = undefined;
				this.#pendingSaveScheduledTimestamp = undefined;
			} else {
				// Current schedule is fine, don't reschedule
				return;
			}
		}

		if (saveDelay > 0) {
			// Schedule save
			this.#pendingSaveScheduledTimestamp = now + saveDelay;
			this.#pendingSaveTimeout = setTimeout(() => {
				this.#pendingSaveTimeout = undefined;
				this.#pendingSaveScheduledTimestamp = undefined;
				this.#savePersistInner();
			}, saveDelay);
		} else {
			// Save immediately
			this.#savePersistInner();
		}
	}

	/**
	 * Clears any pending save timeout.
	 */
	clearPendingSaveTimeout() {
		if (this.#pendingSaveTimeout) {
			clearTimeout(this.#pendingSaveTimeout);
			this.#pendingSaveTimeout = undefined;
			this.#pendingSaveScheduledTimestamp = undefined;
		}
	}

	/**
	 * Waits for any pending write operations to complete.
	 */
	async waitForPendingWrites(): Promise<void> {
		if (this.#persistWriteQueue.runningDrainLoop) {
			await this.#persistWriteQueue.runningDrainLoop;
		}
	}

	// MARK: - Private Helpers

	#validateStateEnabled() {
		if (!this.stateEnabled) {
			throw new errors.StateNotEnabled();
		}
	}

	#handleStateChange(path: string, value: any) {
		const actorStatePath = isStatePath(path);
		const connStatePath = isConnStatePath(path);

		// Validate CBOR serializability
		if (actorStatePath || connStatePath) {
			let invalidPath = "";
			if (
				!isCborSerializable(
					value,
					(invalidPathPart) => {
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

		this.#actor.rLog.debug({
			msg: "onChange triggered, setting persistChanged=true",
			path,
		});
		this.#persistChanged = true;

		// Inform inspector about state changes
		if (actorStatePath) {
			this.#actor.inspector.emitter.emit(
				"stateUpdated",
				this.#persist.state,
			);
		}

		// Call onStateChange lifecycle hook
		if (
			actorStatePath &&
			this.#config.onStateChange &&
			this.#actor.isReady() &&
			!this.#isInOnStateChange
		) {
			try {
				this.#isInOnStateChange = true;
				this.#config.onStateChange(
					this.#actor.actorContext,
					this.#persistRaw.state,
				);
			} catch (error) {
				this.#actor.rLog.error({
					msg: "error in `_onStateChange`",
					error: stringifyError(error),
				});
			} finally {
				this.#isInOnStateChange = false;
			}
		}
	}

	async #savePersistInner() {
		this.#actor.rLog.info({
			msg: "savePersistInner called",
			persistChanged: this.#persistChanged,
			connsWithPersistChangedSize:
				this.#actor.connectionManager.connsWithPersistChanged.size,
			connsWithPersistChangedIds: Array.from(
				this.#actor.connectionManager.connsWithPersistChanged,
			),
		});

		try {
			this.#lastSaveTime = Date.now();

			// Check if either actor state or connections have changed
			const hasChanges =
				this.#persistChanged ||
				this.#actor.connectionManager.connsWithPersistChanged.size > 0;

			if (hasChanges) {
				await this.#persistWriteQueue.enqueue(async () => {
					this.#actor.rLog.debug({
						msg: "saving persist",
						actorChanged: this.#persistChanged,
						connectionsChanged:
							this.#actor.connectionManager
								.connsWithPersistChanged.size,
					});

					const entries: Array<[Uint8Array, Uint8Array]> = [];

					// Build actor entries
					if (this.#persistChanged) {
						this.#persistChanged = false;
						const bareData = convertActorToBarePersisted<S, I>(
							this.#persistRaw,
						);
						entries.push([
							KEYS.PERSIST_DATA,
							ACTOR_VERSIONED.serializeWithEmbeddedVersion(
								bareData,
							),
						]);
					}

					// Build connection entries
					const connections: Array<AnyConn> = [];
					for (const connId of this.#actor.connectionManager
						.connsWithPersistChanged) {
						const conn = this.#actor.conns.get(connId);
						if (!conn) {
							this.#actor.rLog.warn({
								msg: "connection not found in conns map",
								connId,
							});
							continue;
						}

						const connStateManager =
							conn[CONN_STATE_MANAGER_SYMBOL];
						const hibernatableDataRaw =
							connStateManager.hibernatableDataRaw;
						if (!hibernatableDataRaw) {
							this.#actor.log.warn({
								msg: "missing raw hibernatable data for conn in getChangedConnectionsData",
								connId: conn.id,
							});
							continue;
						}

						this.#actor.rLog.info({
							msg: "persisting connection",
							connId,
							hibernatableRequestId:
								hibernatableDataRaw.hibernatableRequestId,
							msgIndex: hibernatableDataRaw.msgIndex,
							hasState: hibernatableDataRaw.state !== undefined,
						});

						const bareData = convertConnToBarePersistedConn<CP, CS>(
							hibernatableDataRaw,
						);
						const connData =
							CONN_VERSIONED.serializeWithEmbeddedVersion(
								bareData,
							);

						entries.push([makeConnKey(connId), connData]);
						connections.push(conn);
					}

					this.#actor.rLog.info({
						msg: "prepared entries for kvBatchPut",
						totalEntries: entries.length,
						connectionEntries: connections.length,
						connectionIds: connections.map((c) => c.id),
					});

					// Notify driver before persisting connections
					if (this.#actorDriver.onBeforePersistConn) {
						for (const conn of connections) {
							this.#actorDriver.onBeforePersistConn(conn);
						}
					}

					// Clear changed connections
					this.#actor.connectionManager.clearConnWithPersistChanged();

					// Write data
					this.#actor.rLog.info({
						msg: "calling kvBatchPut",
						actorId: this.#actor.id,
						entriesCount: entries.length,
					});
					await this.#actorDriver.kvBatchPut(this.#actor.id, entries);
					this.#actor.rLog.info({
						msg: "kvBatchPut completed successfully",
					});

					// Test: Check if KV data is immediately available after write
					try {
						// Try kvListAll first
						if (
							"kvListAll" in this.#actorDriver &&
							typeof this.#actorDriver.kvListAll === "function"
						) {
							const kvEntries = await (
								this.#actorDriver as any
							).kvListAll(this.#actor.id);
							this.#actor.rLog.info({
								msg: "KV verification with kvListAll immediately after write",
								actorId: this.#actor.id,
								entriesFound: kvEntries.length,
								keys: kvEntries.map(
									([k]: [Uint8Array, Uint8Array]) =>
										new TextDecoder().decode(k),
								),
							});
						} else if (
							"kvListPrefix" in this.#actorDriver &&
							typeof this.#actorDriver.kvListPrefix === "function"
						) {
							// Fallback to kvListPrefix if kvListAll doesn't exist
							const kvEntries = await (
								this.#actorDriver as any
							).kvListPrefix(this.#actor.id, new Uint8Array());
							this.#actor.rLog.info({
								msg: "KV verification with kvListPrefix immediately after write",
								actorId: this.#actor.id,
								entriesFound: kvEntries.length,
								keys: kvEntries.map(
									([k]: [Uint8Array, Uint8Array]) =>
										new TextDecoder().decode(k),
								),
							});
						}
					} catch (verifyError) {
						this.#actor.rLog.warn({
							msg: "Failed to verify KV after write",
							error: stringifyError(verifyError),
						});
					}

					// List KV to verify what was written
					// TODO: Re-enable when kvList is implemented on ActorDriver
					// try {
					// 	const kvList = await this.#actorDriver.kvList(this.#actor.id);
					// 	this.#actor.rLog.info({
					// 		msg: "KV list after write",
					// 		keys: kvList.map((k: Uint8Array) => {
					// 			const keyStr = new TextDecoder().decode(k);
					// 			return keyStr;
					// 		}),
					// 		keysCount: kvList.length,
					// 	});
					// } catch (listError) {
					// 	this.#actor.rLog.warn({
					// 		msg: "failed to list KV after write",
					// 		error: stringifyError(listError),
					// 	});
					// }

					// Notify driver after persisting connections
					if (this.#actorDriver.onAfterPersistConn) {
						for (const conn of connections) {
							this.#actorDriver.onAfterPersistConn(conn);
						}
					}

					this.#actor.rLog.debug({ msg: "persist saved" });
				});
			} else {
				this.#actor.rLog.info({
					msg: "savePersistInner skipped - no changes",
				});
			}

			this.#onPersistSavedPromise?.resolve();
		} catch (error) {
			this.#actor.rLog.error({
				msg: "error saving persist",
				error: stringifyError(error),
			});
			this.#onPersistSavedPromise?.reject(error);
			throw error;
		}
	}
}
