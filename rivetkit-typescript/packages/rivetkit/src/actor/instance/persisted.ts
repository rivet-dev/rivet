/**
 * Persisted data structures for actors.
 *
 * Keep this file in sync with the Connection section of rivetkit-typescript/packages/rivetkit/schemas/actor-persist/
 */

import * as cbor from "cbor-x";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { bufferToArrayBuffer } from "@/utils";

export type Cbor = ArrayBuffer;

// MARK: Schedule Event
/** Scheduled event to be executed at a specific timestamp */
export interface PersistedScheduleEvent {
	eventId: string;
	timestamp: number;
	action: string;
	args?: Cbor;
}

// MARK: Actor
/** State object that gets automatically persisted to storage */
export interface PersistedActor<S, I> {
	/** Input data passed to the actor on initialization */
	input?: I;
	hasInitialized: boolean;
	state: S;
	scheduledEvents: PersistedScheduleEvent[];
}

export function convertActorToBarePersisted<S, I>(
	persist: PersistedActor<S, I>,
): persistSchema.Actor {
	return {
		input:
			persist.input !== undefined
				? bufferToArrayBuffer(cbor.encode(persist.input))
				: null,
		hasInitialized: persist.hasInitialized,
		state: bufferToArrayBuffer(cbor.encode(persist.state)),
		scheduledEvents: persist.scheduledEvents.map((event) => ({
			eventId: event.eventId,
			timestamp: BigInt(event.timestamp),
			action: event.action,
			args: event.args ?? null,
		})),
	};
}

export function convertActorFromBarePersisted<S, I>(
	bareData: persistSchema.Actor,
): PersistedActor<S, I> {
	return {
		input: bareData.input
			? cbor.decode(new Uint8Array(bareData.input))
			: undefined,
		hasInitialized: bareData.hasInitialized,
		state: cbor.decode(new Uint8Array(bareData.state)),
		scheduledEvents: bareData.scheduledEvents.map((event) => ({
			eventId: event.eventId,
			timestamp: Number(event.timestamp),
			action: event.action,
			args: event.args ?? undefined,
		})),
	};
}
