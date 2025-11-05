import type { Context as HonoContext } from "hono";
import type { AnyClient } from "@/client/client";
import type { ManagerDriver } from "@/manager/driver";
import type { RegistryConfig } from "@/registry/config";
import type { RunnerConfig } from "@/registry/run-config";
import type { AnyActorInstance } from "./instance";

export type ActorDriverBuilder = (
	registryConfig: RegistryConfig,
	runConfig: RunnerConfig,
	managerDriver: ManagerDriver,
	inlineClient: AnyClient,
) => ActorDriver;

export interface ActorDriver {
	//load(): Promise<LoadOutput>;

	loadActor(actorId: string): Promise<AnyActorInstance>;

	getContext(actorId: string): unknown;

	readPersistedData(actorId: string): Promise<Uint8Array | undefined>;

	/** ActorInstance ensure that only one instance of writePersistedData is called in parallel at a time. */
	writePersistedData(actorId: string, data: Uint8Array): Promise<void>;

	// Schedule
	/** ActorInstance ensure that only one instance of setAlarm is called in parallel at a time. */
	setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void>;

	// Database
	/**
	 * @experimental
	 * This is an experimental API that may change in the future.
	 */
	getDatabase(actorId: string): Promise<unknown | undefined>;

	/**
	 * Requests the actor to go to sleep.
	 *
	 * This will call `_stop` independently.
	 */
	startSleep?(actorId: string): void;

	/**
	 * Shuts down the actor runner.
	 */
	shutdownRunner?(immediate: boolean): Promise<void>;

	// Serverless
	/** This handles the serverless start request. This should manage the lifecycle of the runner tied to the request lifecycle. */
	serverlessHandleStart?(c: HonoContext): Promise<Response>;

	/** Extra properties to add to logs for each actor. */
	getExtraActorLogParams?(): Record<string, string>;
}
