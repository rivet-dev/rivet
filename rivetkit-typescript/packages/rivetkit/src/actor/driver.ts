import type { Context as HonoContext } from "hono";
import type { AnyClient } from "@/client/client";
import type { ManagerDriver } from "@/manager/driver";
import { type AnyConn } from "./conn/mod";
import type { AnyActorInstance } from "./instance/mod";
import type { RegistryConfig } from "@/registry/config";
import type {
	RawDatabaseClient,
} from "@/db/config";
import type { SqliteVfs } from "@rivetkit/sqlite-vfs";
import { BaseSQLiteDatabase } from "drizzle-orm/sqlite-core";

export type ActorDriverBuilder = (
	config: RegistryConfig,
	managerDriver: ManagerDriver,
	inlineClient: AnyClient,
) => ActorDriver;

export interface ActorDriver {
	//load(): Promise<LoadOutput>;

	loadActor(actorId: string): Promise<AnyActorInstance>;

	getContext(actorId: string): unknown;

	// Batch KV operations
	/** Batch write multiple key-value pairs. Keys and values are Uint8Arrays. */
	kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void>;

	/** Batch read multiple keys. Returns null for keys that don't exist. */
	kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]>;

	/** Batch delete multiple keys. */
	kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void>;

	/** List all keys with a given prefix. */
	kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]>;

	// Schedule
	/** ActorInstance ensure that only one instance of setAlarm is called in parallel at a time. */
	setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void>;

	// Database
	/**
	 * Override the default raw database client for the actor.
	 * If not provided, rivetkit will construct a KV-backed SQLite client.
	 * @experimental
	 */
	overrideRawDatabaseClient?(
		actorId: string,
	): Promise<RawDatabaseClient | undefined>;

	/**
	 * Override the default Drizzle database client for the actor.
	 * If not provided, rivetkit will construct a KV-backed Drizzle client.
	 * @experimental
	 */
	overrideDrizzleDatabaseClient?(
		actorId: string,
	): Promise<BaseSQLiteDatabase<any,any,any,any> | undefined>;

	/**
	 * Returns a SQLite VFS instance for creating KV-backed databases.
	 * If not provided, the database provider will need an override.
	 *
	 * @rivetkit/sqlite's async build is not re-entrant per module instance. Drivers
	 * should return a new instance per call for actor-level isolation.
	 *
	 * This is a method (not a property) so drivers can use dynamic imports,
	 * keeping the core driver tree-shakeable from @rivetkit/sqlite.
	 */
	getSqliteVfs?(): SqliteVfs | Promise<SqliteVfs>;

	/**
	 * Requests the actor to go to sleep.
	 *
	 * This will call `ActorInstance.onStop` independently.
	 */
	startSleep?(actorId: string): void;

	/**
	 * Destroys the actor and its associated data.
	 *
	 * This will call `ActorInstance.onStop` independently.
	 */
	startDestroy(actorId: string): void;

	/**
	 * Shuts down the actor runner.
	 */
	shutdownRunner?(immediate: boolean): Promise<void>;

	// Serverless
	/** This handles the serverless start request. This should manage the lifecycle of the runner tied to the request lifecycle. */
	serverlessHandleStart?(c: HonoContext): Promise<Response>;

	/** Extra properties to add to logs for each actor. */
	getExtraActorLogParams?(): Record<string, string>;

	onBeforeActorStart?(actor: AnyActorInstance): Promise<void>;
	onCreateConn?(conn: AnyConn): void;
	onDestroyConn?(conn: AnyConn): void;
	onBeforePersistConn?(conn: AnyConn): void;
	onAfterPersistConn?(conn: AnyConn): void;
}
