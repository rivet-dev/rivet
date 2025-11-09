import * as cbor from "cbor-x";
import type * as protocol from "@/schemas/client-protocol/mod";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import { bufferToArrayBuffer } from "@/utils";
import {
	CONN_PERSIST_SYMBOL,
	CONN_SEND_MESSAGE_SYMBOL,
	type Conn,
} from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import { CachedSerializer } from "../protocol/serde";
import type { ActorInstance } from "./mod";

/**
 * Manages event subscriptions and broadcasting for actor instances.
 * Handles subscription tracking and efficient message distribution to connected clients.
 */
export class EventManager<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	#actor: ActorInstance<S, CP, CS, V, I, DB>;
	#subscriptionIndex = new Map<string, Set<Conn<S, CP, CS, V, I, DB>>>();

	constructor(actor: ActorInstance<S, CP, CS, V, I, DB>) {
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
		connection: Conn<S, CP, CS, V, I, DB>,
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
			connection[CONN_PERSIST_SYMBOL].subscriptions.push({ eventName });

			// Mark connection as changed for persistence
			const connectionManager = (this.#actor as any).connectionManager;
			if (connectionManager) {
				connectionManager.markConnChanged(connection);
			}

			// Save state immediately
			const stateManager = (this.#actor as any).stateManager;
			if (stateManager) {
				stateManager.saveState({ immediate: true });
			}
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
		connection: Conn<S, CP, CS, V, I, DB>,
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
			const subIdx = connection[
				CONN_PERSIST_SYMBOL
			].subscriptions.findIndex((s) => s.eventName === eventName);
			if (subIdx !== -1) {
				connection[CONN_PERSIST_SYMBOL].subscriptions.splice(subIdx, 1);
			} else {
				this.#actor.rLog.warn({
					msg: "subscription does not exist in persist",
					eventName,
					connId: connection.id,
				});
			}

			// Mark connection as changed for persistence
			const connectionManager = (this.#actor as any).connectionManager;
			if (connectionManager) {
				connectionManager.markConnChanged(connection);
			}

			// Save state immediately
			const stateManager = (this.#actor as any).stateManager;
			if (stateManager) {
				stateManager.saveState({ immediate: true });
			}
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
		// Emit to inspector
		this.#actor.inspector.emitter.emit("eventFired", {
			type: "broadcast",
			eventName: name,
			args,
		});

		// Get subscribers for this event
		const subscribers = this.#subscriptionIndex.get(name);
		if (!subscribers || subscribers.size === 0) {
			this.#actor.rLog.debug({
				msg: "no subscribers for event",
				eventName: name,
			});
			return;
		}

		// Create serialized message
		const toClientSerializer = new CachedSerializer<protocol.ToClient>(
			{
				body: {
					tag: "Event",
					val: {
						name,
						args: bufferToArrayBuffer(cbor.encode(args)),
					},
				},
			},
			TO_CLIENT_VERSIONED,
		);

		// Send to all subscribers
		let sentCount = 0;
		for (const connection of subscribers) {
			try {
				connection[CONN_SEND_MESSAGE_SYMBOL](toClientSerializer);
				sentCount++;
			} catch (error) {
				this.#actor.rLog.error({
					msg: "failed to send event to connection",
					eventName: name,
					connId: connection.id,
					error:
						error instanceof Error ? error.message : String(error),
				});
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
	): Set<Conn<S, CP, CS, V, I, DB>> | undefined {
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
	clearConnectionSubscriptions(connection: Conn<S, CP, CS, V, I, DB>) {
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
