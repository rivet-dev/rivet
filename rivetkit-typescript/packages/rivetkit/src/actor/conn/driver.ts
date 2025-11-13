import type { AnyConn } from "@/actor/conn/mod";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { CachedSerializer } from "@/actor/protocol/serde";
import type * as protocol from "@/schemas/client-protocol/mod";

export enum DriverReadyState {
	UNKNOWN = -1,
	CONNECTING = 0,
	OPEN = 1,
	CLOSING = 2,
	CLOSED = 3,
}

export interface ConnDriver {
	/** The type of driver. Used for debug purposes only. */
	type: string;

	/**
	 * Unique request ID provided by the underlying provider. If none is
	 * available for this conn driver, a random UUID is generated.
	 **/
	requestId: string;

	/** ArrayBuffer version of requestId if relevant. */
	requestIdBuf: ArrayBuffer | undefined;

	/**
	 * If the connection can be hibernated. If true, this will allow the actor to go to sleep while the connection is still active.
	 **/
	hibernatable: boolean;

	sendMessage?(
		actor: AnyActorInstance,
		conn: AnyConn,
		message: CachedSerializer<protocol.ToClient>,
	): void;

	/**
	 * This returns a promise since we commonly disconnect at the end of a program, and not waiting will cause the socket to not close cleanly.
	 */
	disconnect(
		actor: AnyActorInstance,
		conn: AnyConn,
		reason?: string,
	): Promise<void>;

	/** Terminates the connection without graceful handling. */
	terminate?(actor: AnyActorInstance, conn: AnyConn): void;

	/**
	 * Returns the ready state of the connection.
	 * This is used to determine if the connection is ready to send messages, or if the connection is stale.
	 */
	getConnectionReadyState(
		actor: AnyActorInstance,
		conn: AnyConn,
	): DriverReadyState | undefined;
}
