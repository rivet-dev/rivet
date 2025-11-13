import * as cbor from "cbor-x";
import onChange from "on-change";
import { isCborSerializable, stringifyError } from "@/common/utils";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import {
	bufferToArrayBuffer,
	promiseWithResolvers,
	SinglePromiseQueue,
} from "@/utils";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { isConnStatePath, isStatePath } from "../utils";
import { KEYS } from "./kv";
import type { ActorInstance } from "./mod";
import type { PersistedActor } from "./persisted";

export interface SaveStateOptions {
	/**
	 * Forces the state to be saved immediately. This function will return when the state has saved successfully.
	 */
	immediate?: boolean;
	/** Bypass ready check for stopping. */
	allowStoppingState?: boolean;
}

/**
 * Manages actor state persistence, proxying, and synchronization.
 * Handles automatic state change detection and throttled persistence to KV storage.
 */
export class StateManager<S, CP, CS, I> {
	#actor: ActorInstance<S, CP, CS, any, I, any>;
	#actorDriver: ActorDriver;

	// State tracking
	#persist!: PersistedActor<S, CP, CS, I>;
	#persistRaw!: PersistedActor<S, CP, CS, I>;
	#persistChanged = false;
	#isInOnStateChange = false;

	// Save management
	#persistWriteQueue = new SinglePromiseQueue();
	#lastSaveTime = 0;
	#pendingSaveTimeout?: NodeJS.Timeout;
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

	get persist(): PersistedActor<S, CP, CS, I> {
		return this.#persist;
	}

	get persistRaw(): PersistedActor<S, CP, CS, I> {
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
	async initializeState(
		persistData: PersistedActor<S, CP, CS, I>,
	): Promise<void> {
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
			await this.#writePersistedDataDirect(persistData);
		}

		// Initialize proxy
		this.initPersistProxy(persistData);
	}

	/**
	 * Creates proxy for persist object that handles automatic state change detection.
	 */
	initPersistProxy(target: PersistedActor<S, CP, CS, I>) {
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
		this.#actor.rLog.debug({
			msg: "saveState called",
			persistChanged: this.#persistChanged,
			allowStoppingState: opts.allowStoppingState,
			immediate: opts.immediate,
		});

		if (this.#persistChanged) {
			if (opts.immediate) {
				await this.#savePersistInner();
			} else {
				// Create promise for waiting
				if (!this.#onPersistSavedPromise) {
					this.#onPersistSavedPromise = promiseWithResolvers();
				}

				// Save throttled
				this.savePersistThrottled();

				// Wait for save
				await this.#onPersistSavedPromise.promise;
			}
		}
	}

	/**
	 * Throttled save state method. Used to write to KV at a reasonable cadence.
	 */
	savePersistThrottled() {
		const now = Date.now();
		const timeSinceLastSave = now - this.#lastSaveTime;

		if (timeSinceLastSave < this.#stateSaveInterval) {
			// Schedule next save if not already scheduled
			if (this.#pendingSaveTimeout === undefined) {
				this.#pendingSaveTimeout = setTimeout(() => {
					this.#pendingSaveTimeout = undefined;
					this.#savePersistInner();
				}, this.#stateSaveInterval - timeSinceLastSave);
			}
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

	/**
	 * Gets persistence data entries if state has changed.
	 */
	getPersistedDataIfChanged(): [Uint8Array, Uint8Array] | null {
		if (!this.#persistChanged) return null;

		this.#persistChanged = false;

		const bareData = this.convertToBarePersisted(this.#persistRaw);
		return [
			KEYS.PERSIST_DATA,
			ACTOR_VERSIONED.serializeWithEmbeddedVersion(bareData),
		];
	}

	// MARK: - BARE Conversion

	convertToBarePersisted(
		persist: PersistedActor<S, CP, CS, I>,
	): persistSchema.Actor {
		const hibernatableConns: persistSchema.HibernatableConn[] =
			persist.hibernatableConns.map((conn) => ({
				id: conn.id,
				parameters: bufferToArrayBuffer(
					cbor.encode(conn.parameters || {}),
				),
				state: bufferToArrayBuffer(cbor.encode(conn.state || {})),
				subscriptions: conn.subscriptions.map((sub) => ({
					eventName: sub.eventName,
				})),
				hibernatableRequestId: conn.hibernatableRequestId,
				lastSeenTimestamp: BigInt(conn.lastSeenTimestamp),
				msgIndex: BigInt(conn.msgIndex),
			}));

		return {
			input:
				persist.input !== undefined
					? bufferToArrayBuffer(cbor.encode(persist.input))
					: null,
			hasInitialized: persist.hasInitialized,
			state: bufferToArrayBuffer(cbor.encode(persist.state)),
			hibernatableConns,
			scheduledEvents: persist.scheduledEvents.map((event) => ({
				eventId: event.eventId,
				timestamp: BigInt(event.timestamp),
				action: event.action,
				args: event.args ?? null,
			})),
		};
	}

	convertFromBarePersisted(
		bareData: persistSchema.Actor,
	): PersistedActor<S, CP, CS, I> {
		const hibernatableConns = bareData.hibernatableConns.map((conn) => ({
			id: conn.id,
			parameters: cbor.decode(new Uint8Array(conn.parameters)),
			state: cbor.decode(new Uint8Array(conn.state)),
			subscriptions: conn.subscriptions.map((sub) => ({
				eventName: sub.eventName,
			})),
			hibernatableRequestId: conn.hibernatableRequestId,
			lastSeenTimestamp: Number(conn.lastSeenTimestamp),
			msgIndex: Number(conn.msgIndex),
		}));

		return {
			input: bareData.input
				? cbor.decode(new Uint8Array(bareData.input))
				: undefined,
			hasInitialized: bareData.hasInitialized,
			state: cbor.decode(new Uint8Array(bareData.state)),
			hibernatableConns,
			scheduledEvents: bareData.scheduledEvents.map((event) => ({
				eventId: event.eventId,
				timestamp: Number(event.timestamp),
				action: event.action,
				args: event.args ?? undefined,
			})),
		};
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
		try {
			this.#lastSaveTime = Date.now();

			if (this.#persistChanged) {
				await this.#persistWriteQueue.enqueue(async () => {
					this.#actor.rLog.debug({
						msg: "saving persist",
						actorChanged: this.#persistChanged,
					});

					const entry = this.getPersistedDataIfChanged();
					if (entry) {
						await this.#actorDriver.kvBatchPut(this.#actor.id, [
							entry,
						]);
					}

					this.#actor.rLog.debug({ msg: "persist saved" });
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

	async #writePersistedDataDirect(persistData: PersistedActor<S, CP, CS, I>) {
		const bareData = this.convertToBarePersisted(persistData);
		await this.#actorDriver.kvBatchPut(this.#actor.id, [
			[
				KEYS.PERSIST_DATA,
				ACTOR_VERSIONED.serializeWithEmbeddedVersion(bareData),
			],
		]);
	}
}
