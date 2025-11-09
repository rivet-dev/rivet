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
	requestId: string;
	requestIdBuf: ArrayBuffer | undefined;
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
