import * as cbor from "cbor-x";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	TO_CLIENT_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type ToClient as ToClientJson,
	ToClientSchema,
} from "@/schemas/client-protocol-zod/mod";
import { bufferToArrayBuffer } from "@/utils";
import {
	CONN_SEND_MESSAGE_SYMBOL,
	CONN_SPEAKS_RIVETKIT_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
	type Conn,
} from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import * as errors from "../errors";
import { CachedSerializer } from "../protocol/serde";
import type { SchemaConfig } from "../schema";
import type { ActorInstance } from "./mod";

/**
 * Manages event subscriptions and broadcasting for actor instances.
 * Handles subscription tracking and efficient message distribution to connected clients.
 */
export class EventManager<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends SchemaConfig = Record<never, never>,
	Q extends SchemaConfig = Record<never, never>,
> {
	#actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>;
	#subscriptionIndex = new Map<
		string,
		Set<Conn<S, CP, CS, V, I, DB, E, Q>>
	>();

	constructor(actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>) {
		this.#actor = actor;
	}

	// MARK: - Public API

	/**
	 * Adds a subscription for a connection to an event.
	 *
	 * @param eventName - The name of the event to subscribe to
	 * @param connection - The connection subscribing to the event
	 * @param fromPersist - Whether this subscription is being restored from persistence
	 */
	addSubscription(
		eventName: string,
		connection: Conn<S, CP, CS, V, I, DB, E, Q>,
		fromPersist: boolean,
	) {
		// Check if already subscribed
		if (connection.subscriptions.has(eventName)) {
			this.#actor.rLog.debug({
				msg: "connection already has subscription",
				eventName,
				connId: connection.id,
			});
			return;
		}

		// Update connection's subscription list
		connection.subscriptions.add(eventName);

		// Update subscription index
		let subscribers = this.#subscriptionIndex.get(eventName);
		if (!subscribers) {
			subscribers = new Set();
			this.#subscriptionIndex.set(eventName, subscribers);
		}
		subscribers.add(connection);

		// Persist subscription if not restoring from persistence
		if (!fromPersist) {
			connection[CONN_STATE_MANAGER_SYMBOL].addSubscription({
				eventName,
			});

			// Save state immediately
			this.#actor.stateManager.saveState({ immediate: true });
		}

		this.#actor.rLog.debug({
			msg: "subscription added",
			eventName,
			connId: connection.id,
			totalSubscribers: subscribers.size,
		});
	}

	/**
	 * Removes a subscription for a connection from an event.
	 *
	 * @param eventName - The name of the event to unsubscribe from
	 * @param connection - The connection unsubscribing from the event
	 * @param fromRemoveConn - Whether this is being called as part of connection removal
	 */
	removeSubscription(
		eventName: string,
		connection: Conn<S, CP, CS, V, I, DB, E, Q>,
		fromRemoveConn: boolean,
	) {
		// Check if subscription exists
		if (!connection.subscriptions.has(eventName)) {
			this.#actor.rLog.warn({
				msg: "connection does not have subscription",
				eventName,
				connId: connection.id,
			});
			return;
		}

		// Remove from connection's subscription list
		connection.subscriptions.delete(eventName);

		// Update subscription index
		const subscribers = this.#subscriptionIndex.get(eventName);
		if (subscribers) {
			subscribers.delete(connection);
			if (subscribers.size === 0) {
				this.#subscriptionIndex.delete(eventName);
			}
		}

		// Update persistence if not part of connection removal
		if (!fromRemoveConn) {
			// Remove from persisted subscriptions
			const removed = connection[
				CONN_STATE_MANAGER_SYMBOL
			].removeSubscription({ eventName });
			if (!removed) {
				this.#actor.rLog.warn({
					msg: "subscription does not exist in persist",
					eventName,
					connId: connection.id,
				});
			}

			// Save state immediately
			this.#actor.stateManager.saveState({ immediate: true });
		}

		this.#actor.rLog.debug({
			msg: "subscription removed",
			eventName,
			connId: connection.id,
			remainingSubscribers: subscribers?.size || 0,
		});
	}

	/**
	 * Broadcasts an event to all subscribed connections.
	 *
	 * @param name - The name of the event to broadcast
	 * @param args - The arguments to send with the event
	 */
	broadcast<Args extends Array<unknown>>(name: string, ...args: Args) {
		this.#actor.assertReady();

		// Get subscribers for this event
		const subscribers = this.#subscriptionIndex.get(name);
		if (!subscribers || subscribers.size === 0) {
			this.#actor.rLog.debug({
				msg: "no subscribers for event",
				eventName: name,
			});
			return;
		}

		this.#actor.emitTraceEvent("message.broadcast", {
			"rivet.event.name": name,
			"rivet.broadcast.subscribers": subscribers.size,
		});

		// Create serialized message
		const eventData = { name, args };
		const toClientSerializer = new CachedSerializer(
			eventData,
			TO_CLIENT_VERSIONED,
			CLIENT_PROTOCOL_CURRENT_VERSION,
			ToClientSchema,
			// JSON: args is the raw value (array of arguments)
			(value): ToClientJson => ({
				body: {
					tag: "Event" as const,
					val: {
						name: value.name,
						args: value.args,
					},
				},
			}),
			// BARE/CBOR: args needs to be CBOR-encoded to ArrayBuffer
			(value): protocol.ToClient => ({
				body: {
					tag: "Event" as const,
					val: {
						name: value.name,
						args: bufferToArrayBuffer(cbor.encode(value.args)),
					},
				},
			}),
		);

		// Send to all subscribers
		let sentCount = 0;
		for (const connection of subscribers) {
			if (connection[CONN_SPEAKS_RIVETKIT_SYMBOL]) {
				try {
					connection[CONN_SEND_MESSAGE_SYMBOL](toClientSerializer);
					sentCount++;
				} catch (error) {
					// Propagate message size errors to the call site so developers
					// can handle them
					if (error instanceof errors.OutgoingMessageTooLong) {
						throw error;
					}
					// Log other errors (e.g., closed connections) and continue
					this.#actor.rLog.error({
						msg: "failed to send event to connection",
						eventName: name,
						connId: connection.id,
						error:
							error instanceof Error
								? error.message
								: String(error),
					});
				}
			}
		}

		this.#actor.rLog.debug({
			msg: "event broadcasted",
			eventName: name,
			subscriberCount: subscribers.size,
			sentCount,
		});
	}

	/**
	 * Gets all subscribers for a specific event.
	 *
	 * @param eventName - The name of the event
	 * @returns Set of connections subscribed to the event, or undefined if no subscribers
	 */
	getSubscribers(
		eventName: string,
	): Set<Conn<S, CP, CS, V, I, DB, E, Q>> | undefined {
		return this.#subscriptionIndex.get(eventName);
	}

	/**
	 * Gets all events and their subscriber counts.
	 *
	 * @returns Map of event names to subscriber counts
	 */
	getEventStats(): Map<string, number> {
		const stats = new Map<string, number>();
		for (const [eventName, subscribers] of this.#subscriptionIndex) {
			stats.set(eventName, subscribers.size);
		}
		return stats;
	}

	/**
	 * Clears all subscriptions for a connection.
	 * Used during connection cleanup.
	 *
	 * @param connection - The connection to clear subscriptions for
	 */
	clearConnectionSubscriptions(connection: Conn<S, CP, CS, V, I, DB, E, Q>) {
		for (const eventName of [...connection.subscriptions.values()]) {
			this.removeSubscription(eventName, connection, true);
		}
	}

	/**
	 * Gets the total number of unique events being subscribed to.
	 */
	get eventCount(): number {
		return this.#subscriptionIndex.size;
	}

	/**
	 * Gets the total number of subscriptions across all events.
	 */
	get totalSubscriptionCount(): number {
		let total = 0;
		for (const subscribers of this.#subscriptionIndex.values()) {
			total += subscribers.size;
		}
		return total;
	}

	/**
	 * Checks if an event has any subscribers.
	 *
	 * @param eventName - The name of the event to check
	 * @returns True if the event has at least one subscriber
	 */
	hasSubscribers(eventName: string): boolean {
		const subscribers = this.#subscriptionIndex.get(eventName);
		return subscribers !== undefined && subscribers.size > 0;
	}
}
