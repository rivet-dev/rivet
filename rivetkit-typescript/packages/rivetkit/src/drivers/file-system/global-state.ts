import invariant from "invariant";
import { lookupInRegistry } from "@/actor/definition";
import { ActorDuplicateKey } from "@/actor/errors";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { ActorKey } from "@/actor/mod";
import type { AnyClient } from "@/client/client";
import { type ActorDriver, getInitialActorKvState } from "@/driver-helpers/mod";
import type { RegistryConfig } from "@/registry/config";
import type * as schema from "@/schemas/file-system-driver/mod";
import {
	ACTOR_ALARM_VERSIONED,
	ACTOR_STATE_VERSIONED,
	CURRENT_VERSION as FILE_SYSTEM_DRIVER_CURRENT_VERSION,
} from "@/schemas/file-system-driver/versioned";
import {
	type LongTimeoutHandle,
	promiseWithResolvers,
	setLongTimeout,
	stringifyError,
} from "@/utils";
import {
	getNodeCrypto,
	getNodeFs,
	getNodeFsSync,
	getNodePath,
} from "@/utils/node";
import { logger } from "./log";
import {
	ensureDirectoryExists,
	ensureDirectoryExistsSync,
	getStoragePath,
} from "./utils";
import {
	computePrefixUpperBound,
	ensureUint8Array,
	loadSqliteRuntime,
	type SqliteRuntime,
	type SqliteRuntimeDatabase,
} from "./sqlite-runtime";
import {
	estimateKvSize,
	validateKvEntries,
	validateKvKey,
	validateKvKeys,
} from "./kv-limits";

// Actor handler to track running instances

enum ActorLifecycleState {
	NONEXISTENT, // Entry exists but actor not yet created
	AWAKE, // Actor is running normally
	STARTING_SLEEP, // Actor is being put to sleep
	STARTING_DESTROY, // Actor is being destroyed
	DESTROYED, // Actor was destroyed, should not be recreated
}

interface ActorEntry {
	id: string;

	state?: schema.ActorState;

	/** Promise for loading the actor state. */
	loadPromise?: Promise<ActorEntry>;

	actor?: AnyActorInstance;
	/** Promise for starting the actor. */
	startPromise?: ReturnType<typeof promiseWithResolvers<void>>;
	/** Promise for stopping the actor. */
	stopPromise?: PromiseWithResolvers<void>;

	alarmTimeout?: LongTimeoutHandle;
	/** The timestamp currently scheduled for this actor's alarm (ms since epoch). */
	alarmTimestamp?: number;

	/** Resolver for pending write operations that need to be notified when any write completes */
	pendingWriteResolver?: PromiseWithResolvers<void>;

	lifecycleState: ActorLifecycleState;

	// TODO: This might make sense to move in to actorstate, but we have a
	// single reader/writer so it's not an issue
	/** Generation of this actor when creating/destroying. */
	generation: string;
}

export interface FileSystemDriverOptions {
	/** Whether to persist data to disk */
	persist?: boolean;
	/** Custom path for storage */
	customPath?: string;
	/** Deprecated option retained for explicit migration to sqlite-only KV. */
	useNativeSqlite?: boolean;
}

/**
 * Global state for the file system driver
 */
export class FileSystemGlobalState {
	#storagePath: string;
	#stateDir: string;
	#dbsDir: string;
	#alarmsDir: string;

	#persist: boolean;
	#sqliteRuntime: SqliteRuntime;
	#actorKvDatabases = new Map<string, SqliteRuntimeDatabase>();

	// IMPORTANT: Never delete from this map. Doing so will result in race
	// conditions since the actor generation will cease to be tracked
	// correctly. Always increment generation if a new actor is created.
	#actors = new Map<string, ActorEntry>();

	#actorCountOnStartup: number = 0;

	#runnerParams?: {
		config: RegistryConfig;
		inlineClient: AnyClient;
		actorDriver: ActorDriver;
	};

	get persist(): boolean {
		return this.#persist;
	}

	get storagePath() {
		return this.#storagePath;
	}

	get actorCountOnStartup() {
		return this.#actorCountOnStartup;
	}

	constructor(options: FileSystemDriverOptions = {}) {
		const { persist = true, customPath, useNativeSqlite = true } = options;
		if (!useNativeSqlite) {
			throw new Error(
				"File-system driver no longer supports non-SQLite KV storage.",
			);
		}
		this.#persist = persist;
		this.#sqliteRuntime = loadSqliteRuntime();
		this.#storagePath = persist ? (customPath ?? getStoragePath()) : "/tmp";
		const path = getNodePath();
		this.#stateDir = path.join(this.#storagePath, "state");
		this.#dbsDir = path.join(this.#storagePath, "databases");
		this.#alarmsDir = path.join(this.#storagePath, "alarms");

		if (this.#persist) {
			// Ensure storage directories exist synchronously during initialization
			ensureDirectoryExistsSync(this.#stateDir);
			ensureDirectoryExistsSync(this.#dbsDir);
			ensureDirectoryExistsSync(this.#alarmsDir);

			try {
				const fsSync = getNodeFsSync();
				const actorIds = fsSync.readdirSync(this.#stateDir);
				this.#actorCountOnStartup = actorIds.length;
			} catch (error) {
				logger().error({ msg: "failed to count actors", error });
			}

			logger().debug({
				msg: "file system driver ready",
				dir: this.#storagePath,
				actorCount: this.#actorCountOnStartup,
				sqliteRuntime: this.#sqliteRuntime.kind,
			});

			// Cleanup stale temp files on startup
			try {
				this.#cleanupTempFilesSync();
			} catch (err) {
				logger().error({
					msg: "failed to cleanup temp files",
					error: err,
				});
			}

			try {
				this.#migrateLegacyKvToSqliteOnStartupSync();
			} catch (error) {
				logger().error({
					msg: "failed legacy kv startup migration",
					error,
				});
				throw error;
			}
		} else {
			logger().debug({
				msg: "memory driver ready",
				sqliteRuntime: this.#sqliteRuntime.kind,
			});
		}
	}

	getActorStatePath(actorId: string): string {
		return getNodePath().join(this.#stateDir, actorId);
	}

	getActorDbPath(actorId: string): string {
		return getNodePath().join(this.#dbsDir, `${actorId}.db`);
	}

	getActorAlarmPath(actorId: string): string {
		return getNodePath().join(this.#alarmsDir, actorId);
	}

	#getActorKvDatabasePath(actorId: string): string {
		if (this.#persist) {
			return this.getActorDbPath(actorId);
		}
		return ":memory:";
	}

	#ensureActorKvTables(db: SqliteRuntimeDatabase): void {
		db.exec(`
			CREATE TABLE IF NOT EXISTS kv (
				key BLOB PRIMARY KEY NOT NULL,
				value BLOB NOT NULL
			)
		`);
	}

	#getOrCreateActorKvDatabase(actorId: string): SqliteRuntimeDatabase {
		const existing = this.#actorKvDatabases.get(actorId);
		if (existing) {
			return existing;
		}

		const dbPath = this.#getActorKvDatabasePath(actorId);
		if (this.#persist) {
			const path = getNodePath();
			ensureDirectoryExistsSync(path.dirname(dbPath));
		}

		let db: SqliteRuntimeDatabase;
		try {
			db = this.#sqliteRuntime.open(dbPath);
		} catch (error) {
			throw new Error(
				`failed to open actor kv database for actor ${actorId} at ${dbPath}: ${error}`,
			);
		}

		this.#ensureActorKvTables(db);
		this.#actorKvDatabases.set(actorId, db);
		return db;
	}

	#closeActorKvDatabase(actorId: string): void {
		const db = this.#actorKvDatabases.get(actorId);
		if (!db) {
			return;
		}

		try {
			db.close();
		} finally {
			this.#actorKvDatabases.delete(actorId);
		}
	}

	#putKvEntriesInDb(
		db: SqliteRuntimeDatabase,
		entries: [Uint8Array, Uint8Array][],
	): void {
		if (entries.length === 0) {
			return;
		}

		db.exec("BEGIN");
		try {
			for (const [key, value] of entries) {
				db.run("INSERT OR REPLACE INTO kv (key, value) VALUES (?, ?)", [
					key,
					value,
				]);
			}
			db.exec("COMMIT");
		} catch (error) {
			try {
				db.exec("ROLLBACK");
			} catch {
				// Ignore rollback errors, original error is more actionable.
			}
			throw error;
		}
	}

	#isKvDbPopulated(db: SqliteRuntimeDatabase): boolean {
		const row = db.get<{ count: number | bigint }>(
			"SELECT COUNT(*) AS count FROM kv",
		);
		const count = row ? Number(row.count) : 0;
		return count > 0;
	}

	#migrateLegacyKvToSqliteOnStartupSync(): void {
		const fsSync = getNodeFsSync();
		if (!fsSync.existsSync(this.#stateDir)) {
			return;
		}

		const actorIds = fsSync
			.readdirSync(this.#stateDir)
			.filter((id) => !id.includes(".tmp."));

		for (const actorId of actorIds) {
			const statePath = this.getActorStatePath(actorId);
			let state: schema.ActorState;
			try {
				const stateBytes = fsSync.readFileSync(statePath);
				state = ACTOR_STATE_VERSIONED.deserializeWithEmbeddedVersion(
					new Uint8Array(stateBytes),
				);
			} catch (error) {
				logger().warn({
					msg: "failed to parse actor state during startup migration",
					actorId,
					error,
				});
				continue;
			}

			if (!state.kvStorage || state.kvStorage.length === 0) {
				continue;
			}

			const dbPath = this.getActorDbPath(actorId);
			const path = getNodePath();
			ensureDirectoryExistsSync(path.dirname(dbPath));
			const db = this.#sqliteRuntime.open(dbPath);
			try {
				this.#ensureActorKvTables(db);
				if (this.#isKvDbPopulated(db)) {
					continue;
				}

				const legacyEntries = state.kvStorage.map((entry) => [
					new Uint8Array(entry.key),
					new Uint8Array(entry.value),
				]) as [Uint8Array, Uint8Array][];
				this.#putKvEntriesInDb(db, legacyEntries);

				logger().info({
					msg: "migrated legacy actor kv storage to sqlite",
					actorId,
					entryCount: legacyEntries.length,
				});
			} finally {
				db.close();
			}
		}
	}

	async *getActorsIterator(params: {
		cursor?: string;
	}): AsyncGenerator<schema.ActorState> {
		let actorIds = Array.from(this.#actors.keys()).sort();

		// Check if state directory exists first
		const fsSync = getNodeFsSync();
		if (fsSync.existsSync(this.#stateDir)) {
			actorIds = fsSync
				.readdirSync(this.#stateDir)
				.filter((id) => !id.includes(".tmp"))
				.sort();
		}

		const startIndex = params.cursor
			? actorIds.indexOf(params.cursor) + 1
			: 0;

		for (let i = startIndex; i < actorIds.length; i++) {
			const actorId = actorIds[i];
			if (!actorId) {
				continue;
			}

			try {
				const state = await this.loadActorStateOrError(actorId);
				yield state;
			} catch (error) {
				logger().error({
					msg: "failed to load actor state",
					actorId,
					error,
				});
			}
		}
	}

	/**
	 * Ensures an entry exists for this actor.
	 *
	 * Used for #createActor and #loadActor.
	 */
	#upsertEntry(actorId: string): ActorEntry {
		let entry = this.#actors.get(actorId);
		if (entry) {
			return entry;
		}

		entry = {
			id: actorId,
			lifecycleState: ActorLifecycleState.NONEXISTENT,
			generation: crypto.randomUUID(),
		};
		this.#actors.set(actorId, entry);
		return entry;
	}

	/**
	 * Creates a new actor and writes to file system.
	 */
	async createActor(
		actorId: string,
		name: string,
		key: ActorKey,
		input: unknown | undefined,
	): Promise<ActorEntry> {
		// TODO: Does not check if actor already exists on fs

		await this.#waitForActorStop(actorId);
		let entry = this.#upsertEntry(actorId);

		// Check if actor already exists (has state or is being stopped)
		if (entry.state) {
			throw new ActorDuplicateKey(name, key);
		}
		if (this.isActorStopping(actorId)) {
			await this.#waitForActorStop(actorId);
			entry = this.#upsertEntry(actorId);
		}

		// If actor was destroyed, reset to NONEXISTENT and increment generation
		if (entry.lifecycleState === ActorLifecycleState.DESTROYED) {
			entry.lifecycleState = ActorLifecycleState.NONEXISTENT;
			entry.generation = crypto.randomUUID();
		}

		// Initialize storage (runtime KV is stored in SQLite; state.kvStorage is legacy-only)
		const initialKvState = getInitialActorKvState(input);

		// Initialize metadata
		await this.#withActorWrite(actorId, async (lockedEntry) => {
			lockedEntry.state = {
				actorId,
				name,
				key,
				createdAt: BigInt(Date.now()),
				kvStorage: [],
				startTs: null,
				connectableTs: null,
				sleepTs: null,
				destroyTs: null,
			};
			lockedEntry.lifecycleState = ActorLifecycleState.AWAKE;
			if (this.#persist) {
				await this.#performWrite(
					actorId,
					lockedEntry.generation,
					lockedEntry.state,
				);
			}
			if (initialKvState.length > 0) {
				const db = this.#getOrCreateActorKvDatabase(actorId);
				this.#putKvEntriesInDb(db, initialKvState);
			}
		});

		return entry;
	}

	/**
	 * Loads the actor from disk or returns the existing actor entry. This will return an entry even if the actor does not actually exist.
	 */
	async loadActor(actorId: string): Promise<ActorEntry> {
		const entry = this.#upsertEntry(actorId);

		// Check if destroyed - don't load from disk
		if (entry.lifecycleState === ActorLifecycleState.DESTROYED) {
			return entry;
		}

		// Check if already loaded
		if (entry.state) {
			return entry;
		}

		// If not persisted, then don't load from FS
		if (!this.#persist) {
			return entry;
		}

		// If state is currently being loaded, wait for it
		if (entry.loadPromise) {
			await entry.loadPromise;
			return entry;
		}

		// Start loading state
		entry.loadPromise = this.loadActorState(entry);
		return entry.loadPromise;
	}

	private async loadActorState(entry: ActorEntry) {
		const stateFilePath = this.getActorStatePath(entry.id);

		// Read & parse file
		try {
			const fs = getNodeFs();
			const stateData = await fs.readFile(stateFilePath);

			const loadedState =
				ACTOR_STATE_VERSIONED.deserializeWithEmbeddedVersion(
					new Uint8Array(stateData),
				);

			// Runtime reads/writes are SQLite-only; legacy kvStorage is for one-time startup migration.
			entry.state = {
				...loadedState,
				kvStorage: [],
			};

			return entry;
		} catch (innerError: any) {
			// File does not exist, meaning the actor does not exist
			if (innerError.code === "ENOENT") {
				entry.loadPromise = undefined;
				return entry;
			}

			// For other errors, throw
			const error = new Error(
				`Failed to load actor state: ${innerError}`,
			);
			throw error;
		}
	}

	async loadOrCreateActor(
		actorId: string,
		name: string,
		key: ActorKey,
		input: unknown | undefined,
	): Promise<ActorEntry> {
		await this.#waitForActorStop(actorId);

		// Attempt to load actor
		const entry = await this.loadActor(actorId);

		// If no state for this actor, then create & write state
		if (!entry.state) {
			if (this.isActorStopping(actorId)) {
				await this.#waitForActorStop(actorId);
				return await this.loadOrCreateActor(actorId, name, key, input);
			}

			// If actor was destroyed, reset to NONEXISTENT and increment generation
			if (entry.lifecycleState === ActorLifecycleState.DESTROYED) {
				entry.lifecycleState = ActorLifecycleState.NONEXISTENT;
				entry.generation = crypto.randomUUID();
			}

				// Initialize storage (runtime KV is stored in SQLite; state.kvStorage is legacy-only)
				const initialKvState = getInitialActorKvState(input);

				await this.#withActorWrite(actorId, async (lockedEntry) => {
					lockedEntry.state = {
						actorId,
						name,
						key: key as readonly string[],
						createdAt: BigInt(Date.now()),
						kvStorage: [],
						startTs: null,
						connectableTs: null,
						sleepTs: null,
						destroyTs: null,
					};
					if (this.#persist) {
						await this.#performWrite(
							actorId,
							lockedEntry.generation,
							lockedEntry.state,
						);
					}
					if (initialKvState.length > 0) {
						const db = this.#getOrCreateActorKvDatabase(actorId);
						this.#putKvEntriesInDb(db, initialKvState);
					}
				});
			}
			return entry;
		}

	async sleepActor(actorId: string) {
		invariant(
			this.#persist,
			"cannot sleep actor with memory driver, must use file system driver",
		);

		// Get the actor. We upsert it even though we're about to destroy it so we have a lock on flagging `destroying` as true.
		const actor = this.#upsertEntry(actorId);
		invariant(actor, `tried to sleep ${actorId}, does not exist`);

		// Check if already destroying
		if (this.isActorStopping(actorId)) {
			return;
		}
		actor.lifecycleState = ActorLifecycleState.STARTING_SLEEP;
		actor.stopPromise = promiseWithResolvers((reason) => logger().warn({ msg: "unhandled actor sleep stop promise rejection", reason }));

		// Wait for actor to fully start before stopping it to avoid race conditions
		if (actor.loadPromise) await actor.loadPromise.catch();
		if (actor.startPromise?.promise)
			await actor.startPromise.promise.catch();

		try {
			// Update state with sleep timestamp
			if (actor.state) {
				await this.#withActorWrite(actorId, async (lockedEntry) => {
					if (!lockedEntry.state) {
						return;
					}
					lockedEntry.state = {
						...lockedEntry.state,
						sleepTs: BigInt(Date.now()),
					};
					if (this.#persist) {
						await this.#performWrite(
							actorId,
							lockedEntry.generation,
							lockedEntry.state,
						);
					}
				});
			}

			// Stop actor
			invariant(actor.actor, "actor should be loaded");
			await actor.actor.onStop("sleep");
			} finally {
				// Ensure any pending KV writes finish before removing the entry.
				await this.#withActorWrite(actorId, async () => {});
				this.#closeActorKvDatabase(actorId);
				actor.stopPromise?.resolve();
				actor.stopPromise = undefined;

			// Remove from map after stop is complete
			this.#actors.delete(actorId);
		}
	}

	async destroyActor(actorId: string) {
		// Get the actor. We upsert it even though we're about to destroy it so we have a lock on flagging `destroying` as true.
		const actor = this.#upsertEntry(actorId);

		// If actor is loaded, stop it first
		// Check if already destroying
		if (this.isActorStopping(actorId)) {
			return;
		}
		actor.lifecycleState = ActorLifecycleState.STARTING_DESTROY;
		actor.stopPromise = promiseWithResolvers((reason) => logger().warn({ msg: "unhandled actor destroy stop promise rejection", reason }));

		// Wait for actor to fully start before stopping it to avoid race conditions
		if (actor.loadPromise) await actor.loadPromise.catch();
		if (actor.startPromise?.promise)
			await actor.startPromise.promise.catch();

		try {
			// Update state with destroy timestamp
			if (actor.state) {
				await this.#withActorWrite(actorId, async (lockedEntry) => {
					if (!lockedEntry.state) {
						return;
					}
					lockedEntry.state = {
						...lockedEntry.state,
						destroyTs: BigInt(Date.now()),
					};
					if (this.#persist) {
						await this.#performWrite(
							actorId,
							lockedEntry.generation,
							lockedEntry.state,
						);
					}
				});
			}

			// Stop actor if it's running
			if (actor.actor) {
				await actor.actor.onStop("destroy");
			}

				// Ensure any pending KV writes finish before deleting files.
				await this.#withActorWrite(actorId, async () => {});
				this.#closeActorKvDatabase(actorId);

			// Clear alarm timeout if exists
			if (actor.alarmTimeout) {
				actor.alarmTimeout.abort();
			}

			// Delete persisted files if using file system driver
			if (this.#persist) {
				const fs = getNodeFs();

				// Delete all actor files in parallel
				await Promise.all([
					// Delete actor state file
					(async () => {
						try {
							await fs.unlink(this.getActorStatePath(actorId));
						} catch (err: any) {
							if (err?.code !== "ENOENT") {
								logger().error({
									msg: "failed to delete actor state file",
									actorId,
									error: stringifyError(err),
								});
							}
						}
					})(),
					// Delete actor database file
					(async () => {
						try {
							await fs.unlink(this.getActorDbPath(actorId));
						} catch (err: any) {
							if (err?.code !== "ENOENT") {
								logger().error({
									msg: "failed to delete actor database file",
									actorId,
									error: stringifyError(err),
								});
							}
						}
					})(),
					// Delete actor alarm file
					(async () => {
						try {
							await fs.unlink(this.getActorAlarmPath(actorId));
						} catch (err: any) {
							if (err?.code !== "ENOENT") {
								logger().error({
									msg: "failed to delete actor alarm file",
									actorId,
									error: stringifyError(err),
								});
							}
						}
					})(),
				]);
			}
		} finally {
			// Ensure any pending KV writes finish before clearing the entry.
			await this.#withActorWrite(actorId, async () => {});
			actor.stopPromise?.resolve();
			actor.stopPromise = undefined;

			// Reset the entry
			//
			// Do not remove entry in order to avoid race condition with
			// destroying. Next actor creation will increment the generation.
			actor.state = undefined;
			actor.loadPromise = undefined;
			actor.actor = undefined;
			actor.startPromise = undefined;
			actor.alarmTimeout = undefined;
			actor.alarmTimeout = undefined;
			actor.pendingWriteResolver = undefined;
			actor.lifecycleState = ActorLifecycleState.DESTROYED;
		}
	}

	/**
	 * Save actor state to disk.
	 */
	async writeActor(
		actorId: string,
		generation: string,
		state: schema.ActorState,
	): Promise<void> {
		if (!this.#persist) {
			return;
		}

		await this.#withActorWrite(actorId, async () => {
			await this.#performWrite(actorId, generation, state);
		});
	}

	isGenerationCurrentAndNotDestroyed(
		actorId: string,
		generation: string,
	): boolean {
		const entry = this.#upsertEntry(actorId);
		if (!entry) return false;
		return (
			entry.generation === generation &&
			entry.lifecycleState !== ActorLifecycleState.STARTING_DESTROY
		);
	}

	isActorStopping(actorId: string) {
		const entry = this.#upsertEntry(actorId);
		if (!entry) return false;
		return (
			entry.lifecycleState === ActorLifecycleState.STARTING_SLEEP ||
			entry.lifecycleState === ActorLifecycleState.STARTING_DESTROY
		);
	}

	async #waitForActorStop(actorId: string): Promise<void> {
		while (true) {
			const entry = this.#actors.get(actorId);
			if (!entry?.stopPromise) {
				return;
			}
			try {
				await entry.stopPromise.promise;
			} catch {
				return;
			}
		}
	}

	async #withActorWrite<T>(
		actorId: string,
		fn: (entry: ActorEntry) => Promise<T>,
	): Promise<T> {
		const entry = this.#actors.get(actorId);
		invariant(entry, "actor entry does not exist");

		const previousWrite = entry.pendingWriteResolver;
		const currentWrite = promiseWithResolvers<void>((reason) => logger().warn({ msg: "unhandled kv write promise rejection", reason }));
		entry.pendingWriteResolver = currentWrite;

		if (previousWrite) {
			try {
				await previousWrite.promise;
			} catch {
				// Ignore failed previous writes so later writes can proceed.
			}
		}

		try {
			return await fn(entry);
		} finally {
			currentWrite.resolve();
			if (entry.pendingWriteResolver === currentWrite) {
				entry.pendingWriteResolver = undefined;
			}
		}
	}

	async #waitForPendingWrite(actorId: string): Promise<void> {
		const entry = this.#actors.get(actorId);
		if (!entry?.pendingWriteResolver) {
			return;
		}

		while (entry.pendingWriteResolver) {
			const pending = entry.pendingWriteResolver;
			try {
				await pending.promise;
			} catch {
				// Ignore write failures to avoid blocking reads forever.
			}
		}
	}

	async setActorAlarm(actorId: string, timestamp: number) {
		const entry = this.#actors.get(actorId);
		invariant(entry, "actor entry does not exist");

		// Track generation of the actor when the write started to detect
		// destroy/create race condition
		const writeGeneration = entry.generation;
		if (this.isActorStopping(actorId)) {
			logger().info("skipping set alarm since actor stopping");
			return;
		}

		// Persist alarm to disk
		if (this.#persist) {
			const alarmPath = this.getActorAlarmPath(actorId);
			const crypto = getNodeCrypto();
			const tempPath = `${alarmPath}.tmp.${crypto.randomUUID()}`;
			try {
				const path = getNodePath();
				await ensureDirectoryExists(path.dirname(alarmPath));
				const alarmData: schema.ActorAlarm = {
					actorId,
					timestamp: BigInt(timestamp),
				};
				const data = ACTOR_ALARM_VERSIONED.serializeWithEmbeddedVersion(
					alarmData,
					FILE_SYSTEM_DRIVER_CURRENT_VERSION,
				);
				const fs = getNodeFs();
				await fs.writeFile(tempPath, data);

				if (
					!this.isGenerationCurrentAndNotDestroyed(
						actorId,
						writeGeneration,
					)
				) {
					logger().debug(
						"skipping writing alarm since actor destroying or new generation",
					);
					return;
				}

				await fs.rename(tempPath, alarmPath);
			} catch (error) {
				try {
					const fs = getNodeFs();
					await fs.unlink(tempPath);
				} catch {}
				logger().error({
					msg: "failed to write alarm",
					actorId,
					error,
				});
				throw new Error(`Failed to write alarm: ${error}`);
			}
		}

		// Schedule timeout
		this.#scheduleAlarmTimeout(actorId, timestamp);
	}

	/**
	 * Perform the actual write operation with atomic writes
	 */
	async #performWrite(
		actorId: string,
		generation: string,
		state: schema.ActorState,
	): Promise<void> {
		const dataPath = this.getActorStatePath(actorId);
		// Generate unique temp filename to prevent any race conditions
		const crypto = getNodeCrypto();
		const tempPath = `${dataPath}.tmp.${crypto.randomUUID()}`;

		try {
			// Create directory if needed
			const path = getNodePath();
			await ensureDirectoryExists(path.dirname(dataPath));

			// Convert to BARE types for serialization
			const bareState: schema.ActorState = {
				actorId: state.actorId,
				name: state.name,
				key: state.key,
				createdAt: state.createdAt,
				kvStorage: state.kvStorage,
				startTs: state.startTs,
				connectableTs: state.connectableTs,
				sleepTs: state.sleepTs,
				destroyTs: state.destroyTs,
			};

			// Perform atomic write
			const serializedState =
				ACTOR_STATE_VERSIONED.serializeWithEmbeddedVersion(
					bareState,
					FILE_SYSTEM_DRIVER_CURRENT_VERSION,
				);
			const fs = getNodeFs();
			await fs.writeFile(tempPath, serializedState);

			if (!this.isGenerationCurrentAndNotDestroyed(actorId, generation)) {
				logger().debug(
					"skipping writing alarm since actor destroying or new generation",
				);
				return;
			}

			await fs.rename(tempPath, dataPath);
		} catch (error) {
			// Cleanup temp file on error
			try {
				const fs = getNodeFs();
				await fs.unlink(tempPath);
			} catch {
				// Ignore cleanup errors
			}
			logger().error({
				msg: "failed to save actor state",
				actorId,
				error,
			});
			throw new Error(`Failed to save actor state: ${error}`);
		}
	}

	/**
	 * Call this method after the actor driver has been initiated.
	 *
	 * This will trigger all initial alarms from the file system.
	 *
	 * This needs to be sync since DriverConfig.actor is sync
	 */
	onRunnerStart(
		config: RegistryConfig,
		inlineClient: AnyClient,
		actorDriver: ActorDriver,
	) {
		if (this.#runnerParams) {
			return;
		}

		// Save runner params for future use
		this.#runnerParams = {
			config: config,
			inlineClient,
			actorDriver,
		};

		// Load alarms from disk and schedule timeouts
		try {
			this.#loadAlarmsSync();
		} catch (err) {
			logger().error({
				msg: "failed to load alarms on startup",
				error: err,
			});
		}
	}

	async startActor(
		config: RegistryConfig,
		inlineClient: AnyClient,
		actorDriver: ActorDriver,
		actorId: string,
	): Promise<AnyActorInstance> {
		await this.#waitForActorStop(actorId);

		// Get the actor metadata
		let entry = await this.loadActor(actorId);
		if (!entry.state) {
			throw new Error(
				`Actor does not exist and cannot be started: "${actorId}"`,
			);
		}

		// Actor already starting
		if (entry.startPromise) {
			await entry.startPromise.promise;
			invariant(entry.actor, "actor should have loaded");
			return entry.actor;
		}

		// Actor already loaded
		if (entry.actor) {
			if (entry.actor.isStopping || this.isActorStopping(actorId)) {
				await this.#waitForActorStop(actorId);
				entry = await this.loadActor(actorId);
				if (!entry.state) {
					throw new Error(
						`Actor does not exist and cannot be started: "${actorId}"`,
					);
				}
			} else {
				return entry.actor;
			}
		}

		// Create start promise
		entry.startPromise = promiseWithResolvers((reason) => logger().warn({ msg: "unhandled actor start promise rejection", reason }));

		try {
			// Create actor
			const definition = lookupInRegistry(config, entry.state.name);
			entry.actor = await definition.instantiate();
			entry.lifecycleState = ActorLifecycleState.AWAKE;

			// Start actor
			await entry.actor.start(
				actorDriver,
				inlineClient,
				actorId,
				entry.state.name,
				entry.state.key as string[],
				"unknown",
			);

			// Update state with start timestamp
			// NOTE: connectableTs is always in sync with startTs since actors become connectable immediately after starting
			const now = BigInt(Date.now());
			await this.#withActorWrite(actorId, async (lockedEntry) => {
				if (!lockedEntry.state) {
					throw new Error(
						`Actor does not exist and cannot be started: "${actorId}"`,
					);
				}
				lockedEntry.state = {
					...lockedEntry.state,
					startTs: now,
					connectableTs: now,
					sleepTs: null, // Clear sleep timestamp when actor wakes up
				};
					if (this.#persist) {
						await this.#performWrite(
							actorId,
							lockedEntry.generation,
							lockedEntry.state,
						);
					}
				});

			// Finish
			entry.startPromise.resolve();
			entry.startPromise = undefined;

			return entry.actor;
			} catch (innerError) {
				const error = new Error(
					`Failed to start actor ${actorId}: ${innerError}`,
					{ cause: innerError },
				);
			entry.startPromise?.reject(error);
			entry.startPromise = undefined;
			throw error;
		}
	}

	async loadActorStateOrError(actorId: string): Promise<schema.ActorState> {
		const state = (await this.loadActor(actorId)).state;
		if (!state) throw new Error(`Actor does not exist: ${actorId}`);
		return state;
	}

	getActorOrError(actorId: string): ActorEntry {
		const entry = this.#actors.get(actorId);
		if (!entry) throw new Error(`No entry for actor: ${actorId}`);
		return entry;
	}

	async createDatabase(actorId: string): Promise<string | undefined> {
		return this.getActorDbPath(actorId);
	}

	/**
	 * Load all persisted alarms from disk and schedule their timers.
	 */
	#loadAlarmsSync(): void {
		try {
			const fsSync = getNodeFsSync();
			const files = fsSync.existsSync(this.#alarmsDir)
				? fsSync.readdirSync(this.#alarmsDir)
				: [];
			for (const file of files) {
				// Skip temp files
				if (file.includes(".tmp.")) continue;
				const path = getNodePath();
				const fullPath = path.join(this.#alarmsDir, file);
				try {
					const buf = fsSync.readFileSync(fullPath);
					const alarmData =
						ACTOR_ALARM_VERSIONED.deserializeWithEmbeddedVersion(
							new Uint8Array(buf),
						);
					const timestamp = Number(alarmData.timestamp);
					if (Number.isFinite(timestamp)) {
						this.#scheduleAlarmTimeout(
							alarmData.actorId,
							timestamp,
						);
					} else {
						logger().debug({
							msg: "invalid alarm file contents",
							file,
						});
					}
				} catch (err) {
					logger().error({
						msg: "failed to read alarm file",
						file,
						error: stringifyError(err),
					});
				}
			}
		} catch (err) {
			logger().error({
				msg: "failed to list alarms directory",
				error: err,
			});
		}
	}

	/**
	 * Schedule an alarm timer for an actor without writing to disk.
	 */
	#scheduleAlarmTimeout(actorId: string, timestamp: number) {
		const entry = this.#upsertEntry(actorId);

		// If there's already an earlier alarm scheduled, do not override it.
		if (
			entry.alarmTimestamp !== undefined &&
			timestamp >= entry.alarmTimestamp
		) {
			logger().debug({
				msg: "skipping alarm schedule (later than existing)",
				actorId,
				timestamp,
				current: entry.alarmTimestamp,
			});
			return;
		}

		logger().debug({ msg: "scheduling alarm", actorId, timestamp });

		// Cancel existing timeout and update the current scheduled timestamp
		entry.alarmTimeout?.abort();
		entry.alarmTimestamp = timestamp;

		const delay = Math.max(0, timestamp - Date.now());
		entry.alarmTimeout = setLongTimeout(async () => {
			// Clear currently scheduled timestamp as this alarm is firing now
			entry.alarmTimestamp = undefined;
			// On trigger: remove persisted alarm file
			if (this.#persist) {
				try {
					const fs = getNodeFs();
					await fs.unlink(this.getActorAlarmPath(actorId));
				} catch (err: any) {
					if (err?.code !== "ENOENT") {
						logger().debug({
							msg: "failed to remove alarm file",
							actorId,
							error: stringifyError(err),
						});
					}
				}
			}

			try {
				logger().debug({ msg: "triggering alarm", actorId, timestamp });

				// Ensure actor state exists and start actor if needed
				const loaded = await this.loadActor(actorId);
				if (!loaded.state)
					throw new Error(`Actor does not exist: ${actorId}`);

				// Start actor if not already running
				const runnerParams = this.#runnerParams;
				invariant(runnerParams, "missing runner params");
				if (!loaded.actor) {
					await this.startActor(
						runnerParams.config,
						runnerParams.inlineClient,
						runnerParams.actorDriver,
						actorId,
					);
				}

				invariant(loaded.actor, "actor should be loaded after wake");
				await loaded.actor.onAlarm();
			} catch (err) {
				logger().error({
					msg: "failed to handle alarm",
					actorId,
					error: stringifyError(err),
				});
			}
		}, delay);
	}

	/**
	 * Cleanup stale temp files on startup (synchronous)
	 */
	#cleanupTempFilesSync(): void {
		try {
			const fsSync = getNodeFsSync();
			const files = fsSync.readdirSync(this.#stateDir);
			const tempFiles = files.filter((f) => f.includes(".tmp."));

			const oneHourAgo = Date.now() - 3600000; // 1 hour in ms

			for (const tempFile of tempFiles) {
				try {
					const path = getNodePath();
					const fullPath = path.join(this.#stateDir, tempFile);
					const stat = fsSync.statSync(fullPath);

					// Remove if older than 1 hour
					if (stat.mtimeMs < oneHourAgo) {
						fsSync.unlinkSync(fullPath);
						logger().info({
							msg: "cleaned up stale temp file",
							file: tempFile,
						});
					}
				} catch (err) {
					logger().debug({
						msg: "failed to cleanup temp file",
						file: tempFile,
						error: err,
					});
				}
			}
		} catch (err) {
			logger().error({
				msg: "failed to read actors directory for cleanup",
				error: err,
			});
		}
	}

	/**
	 * Batch put KV entries for an actor.
	 */
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		await this.loadActor(actorId);
		await this.#withActorWrite(actorId, async (entry) => {
			if (!entry.state) {
				if (this.isActorStopping(actorId)) {
					return;
				}
				throw new Error(`Actor ${actorId} state not loaded`);
			}

			const db = this.#getOrCreateActorKvDatabase(actorId);
			const totalSize = estimateKvSize(db);
			validateKvEntries(entries, totalSize);
			this.#putKvEntriesInDb(db, entries);
		});
	}

	/**
	 * Batch get KV entries for an actor.
	 */
	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		const entry = await this.loadActor(actorId);
		await this.#waitForPendingWrite(actorId);
		if (!entry.state) {
			if (this.isActorStopping(actorId)) {
				throw new Error(`Actor ${actorId} is stopping`);
			} else {
				throw new Error(`Actor ${actorId} state not loaded`);
			}
		}

		validateKvKeys(keys);

		const db = this.#getOrCreateActorKvDatabase(actorId);
		const results: (Uint8Array | null)[] = [];
		for (const key of keys) {
			const row = db.get<{ value: Uint8Array | ArrayBuffer }>(
				"SELECT value FROM kv WHERE key = ?",
				[key],
			);
			if (!row) {
				results.push(null);
				continue;
			}
			results.push(ensureUint8Array(row.value, "value"));
		}
		return results;
	}

	/**
	 * Batch delete KV entries for an actor.
	 */
	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		await this.loadActor(actorId);
		await this.#withActorWrite(actorId, async (entry) => {
			if (!entry.state) {
				if (this.isActorStopping(actorId)) {
					return;
				}
				throw new Error(`Actor ${actorId} state not loaded`);
			}

			if (keys.length === 0) {
				return;
			}
			validateKvKeys(keys);

			const db = this.#getOrCreateActorKvDatabase(actorId);
			db.exec("BEGIN");
			try {
				for (const key of keys) {
					db.run("DELETE FROM kv WHERE key = ?", [key]);
				}
				db.exec("COMMIT");
			} catch (error) {
				try {
					db.exec("ROLLBACK");
				} catch {
					// Ignore rollback errors, original error is more actionable.
				}
				throw error;
			}
		});
	}

	/**
	 * List KV entries with a given prefix for an actor.
	 */
	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		const entry = await this.loadActor(actorId);
		await this.#waitForPendingWrite(actorId);
		if (!entry.state) {
			if (this.isActorStopping(actorId)) {
				throw new Error(`Actor ${actorId} is destroying`);
			} else {
				throw new Error(`Actor ${actorId} state not loaded`);
			}
		}
		validateKvKey(prefix, "prefix key");

		const db = this.#getOrCreateActorKvDatabase(actorId);
		const upperBound = computePrefixUpperBound(prefix);
		const rows = upperBound
			? db.all<{ key: Uint8Array | ArrayBuffer; value: Uint8Array | ArrayBuffer }>(
					"SELECT key, value FROM kv WHERE key >= ? AND key < ? ORDER BY key ASC",
					[prefix, upperBound],
				)
			: db.all<{ key: Uint8Array | ArrayBuffer; value: Uint8Array | ArrayBuffer }>(
					"SELECT key, value FROM kv WHERE key >= ? ORDER BY key ASC",
					[prefix],
				);

		return rows.map((row) => [
			ensureUint8Array(row.key, "key"),
			ensureUint8Array(row.value, "value"),
		]);
	}
}
