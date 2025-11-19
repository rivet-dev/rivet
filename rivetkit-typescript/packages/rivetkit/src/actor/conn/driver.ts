import type { AnyConn } from "@/actor/conn/mod";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { CachedSerializer } from "@/actor/protocol/serde";

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
	 * If defined, this connection driver talks the RivetKit client driver (see
	 * schemas/client-protocol/).
	 *
	 * If enabled, events like `Init`, subscription events, etc. will be sent
	 * to this connection.
	 */
	rivetKitProtocol?: {
		/** Sends a RivetKit client message. */
		sendMessage(
			actor: AnyActorInstance,
			conn: AnyConn,
			message: CachedSerializer<any, any, any>,
		): void;
	};

	/**
	 * If the connection can be hibernated. If true, this will allow the actor to go to sleep while the connection is still active.
	 **/
	hibernatable?: {
		gatewayId: ArrayBuffer;
		requestId: ArrayBuffer;
	};

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
