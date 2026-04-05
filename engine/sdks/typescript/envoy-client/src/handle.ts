import * as protocol from "@rivetkit/engine-envoy-protocol";
import { ActorEntry } from "./tasks/envoy";

export interface KvListOptions {
	reverse?: boolean;
	limit?: number;
}

export interface EnvoyHandle {
	/** Starts the shutdown procedure for this envoy. */
	shutdown(immediate: boolean): void;

	getProtocolMetadata(): protocol.ProtocolMetadata | undefined;

	getEnvoyKey(): string;

	getActor(actorId: string, generation?: number): ActorEntry | undefined;

	/** Send sleep intent for an actor. */
	sleepActor(actorId: string, generation?: number): void;

	/** Send stop intent for an actor. */
	stopActor(actorId: string, generation?: number): void;

	/**
	 * Like stopActor but ensures the actor is fully destroyed rather than
	 * potentially being kept for hibernation.
	 */
	destroyActor(actorId: string, generation?: number): void;

	/** Set or clear an alarm for an actor. Pass null to clear. */
	setAlarm(
		actorId: string,
		alarmTs: number | null,
		generation?: number,
	): void;

	/** Get values for the given keys. Returns null for missing keys. */
	kvGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]>;

	/** List all key-value pairs. */
	kvListAll(
		actorId: string,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;

	/** List key-value pairs within a key range. */
	kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		exclusive?: boolean,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;

	/** List key-value pairs matching a prefix. */
	kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;

	/** Put key-value pairs. */
	kvPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void>;

	/** Delete specific keys. */
	kvDelete(actorId: string, keys: Uint8Array[]): Promise<void>;

	/** Delete a range of keys. */
	kvDeleteRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void>;

	/** Drop all key-value data for an actor. */
	kvDrop(actorId: string): Promise<void>;
}
