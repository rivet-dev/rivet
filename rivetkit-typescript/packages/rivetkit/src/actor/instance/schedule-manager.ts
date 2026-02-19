import * as cbor from "cbor-x";
import {
	bufferToArrayBuffer,
	SinglePromiseQueue,
	stringifyError,
} from "@/utils";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDriver } from "../driver";
import type { SchemaConfig } from "../schema";
import type { ActorInstance } from "./mod";
import type { PersistedScheduleEvent } from "./persisted";

/**
 * Manages scheduled events and alarms for actor instances.
 * Handles event scheduling, alarm triggers, and automatic event execution.
 */
export class ScheduleManager<
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
	#actorDriver: ActorDriver;
	#alarmWriteQueue = new SinglePromiseQueue();
	#config: any; // ActorConfig type
	#persist: any; // Reference to PersistedActor

	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>,
		actorDriver: ActorDriver,
		config: any,
	) {
		this.#actor = actor;
		this.#actorDriver = actorDriver;
		this.#config = config;
	}

	// MARK: - Public API

	/**
	 * Sets the persist object reference.
	 * Called after StateManager initializes the persist proxy.
	 */
	setPersist(persist: any) {
		this.#persist = persist;
	}

	/**
	 * Schedules an event to be executed at a specific timestamp.
	 *
	 * @param timestamp - Unix timestamp in milliseconds when the event should fire
	 * @param action - The name of the action to execute
	 * @param args - Arguments to pass to the action
	 */
	async scheduleEvent(
		timestamp: number,
		action: string,
		args: unknown[],
	): Promise<void> {
		const newEvent: PersistedScheduleEvent = {
			eventId: crypto.randomUUID(),
			timestamp,
			action,
			args: bufferToArrayBuffer(cbor.encode(args)),
		};

		this.#actor.emitTraceEvent("schedule.created", {
			"rivet.schedule.event_id": newEvent.eventId,
			"rivet.schedule.action": newEvent.action,
			"rivet.schedule.timestamp_ms": newEvent.timestamp,
		});

		await this.#scheduleEventInner(newEvent);
	}

	/**
	 * Triggers any pending alarms that are due.
	 * This method is idempotent and safe to call multiple times.
	 */
	async onAlarm(): Promise<void> {
		const now = Date.now();
		this.#actor.log.debug({
			msg: "alarm triggered",
			now,
			events: this.#persist?.scheduledEvents?.length || 0,
		});

		if (!this.#persist?.scheduledEvents) {
			this.#actor.rLog.debug({ msg: "no scheduled events" });
			return;
		}

		// Find events that are due
		const dueIndex = this.#persist.scheduledEvents.findIndex(
			(x: PersistedScheduleEvent) => x.timestamp <= now,
		);

		if (dueIndex === -1) {
			// No events are due yet
			this.#actor.rLog.debug({ msg: "no events are due yet" });

			// Reschedule alarm for next event if any exist
			if (this.#persist.scheduledEvents.length > 0) {
				const nextTs = this.#persist.scheduledEvents[0].timestamp;
				this.#actor.log.debug({
					msg: "alarm fired early, rescheduling for next event",
					now,
					nextTs,
					delta: nextTs - now,
				});
				await this.#queueSetAlarm(nextTs);
			}
			return;
		}

		// Remove and process due events
		const dueEvents = this.#persist.scheduledEvents.splice(0, dueIndex + 1);
		this.#actor.log.debug({
			msg: "running events",
			count: dueEvents.length,
		});

		// Schedule next alarm if more events remain
		if (this.#persist.scheduledEvents.length > 0) {
			const nextTs = this.#persist.scheduledEvents[0].timestamp;
			this.#actor.log.info({
				msg: "setting next alarm",
				nextTs,
				remainingEvents: this.#persist.scheduledEvents.length,
			});
			await this.#queueSetAlarm(nextTs);
		}

		// Execute due events
		await this.#executeDueEvents(dueEvents);
	}

	/**
	 * Initializes alarms on actor startup.
	 * Sets the alarm for the next scheduled event if any exist.
	 */
	async initializeAlarms(): Promise<void> {
		if (this.#persist?.scheduledEvents?.length > 0) {
			await this.#queueSetAlarm(
				this.#persist.scheduledEvents[0].timestamp,
			);
		}
	}

	/**
	 * Waits for any pending alarm write operations to complete.
	 */
	async waitForPendingAlarmWrites(): Promise<void> {
		if (this.#alarmWriteQueue.runningDrainLoop) {
			await this.#alarmWriteQueue.runningDrainLoop;
		}
	}

	/**
	 * Gets statistics about scheduled events.
	 */
	getScheduleStats(): {
		totalEvents: number;
		nextEventTime: number | null;
		overdueCount: number;
	} {
		if (!this.#persist?.scheduledEvents) {
			return {
				totalEvents: 0,
				nextEventTime: null,
				overdueCount: 0,
			};
		}

		const now = Date.now();
		const events = this.#persist.scheduledEvents;

		return {
			totalEvents: events.length,
			nextEventTime: events.length > 0 ? events[0].timestamp : null,
			overdueCount: events.filter(
				(e: PersistedScheduleEvent) => e.timestamp <= now,
			).length,
		};
	}

	/**
	 * Cancels a scheduled event by its ID.
	 *
	 * @param eventId - The ID of the event to cancel
	 * @returns True if the event was found and cancelled
	 */
	async cancelEvent(eventId: string): Promise<boolean> {
		if (!this.#persist?.scheduledEvents) {
			return false;
		}

		const index = this.#persist.scheduledEvents.findIndex(
			(e: PersistedScheduleEvent) => e.eventId === eventId,
		);

		if (index === -1) {
			return false;
		}

		// Remove the event
		const wasFirst = index === 0;
		this.#persist.scheduledEvents.splice(index, 1);

		// If we removed the first event, update the alarm
		if (wasFirst && this.#persist.scheduledEvents.length > 0) {
			await this.#queueSetAlarm(
				this.#persist.scheduledEvents[0].timestamp,
			);
		}

		this.#actor.log.info({
			msg: "cancelled scheduled event",
			eventId,
			remainingEvents: this.#persist.scheduledEvents.length,
		});

		return true;
	}

	// MARK: - Private Helpers

	async #scheduleEventInner(newEvent: PersistedScheduleEvent): Promise<void> {
		this.#actor.log.info({
			msg: "scheduling event",
			eventId: newEvent.eventId,
			timestamp: newEvent.timestamp,
			action: newEvent.action,
		});

		if (!this.#persist?.scheduledEvents) {
			throw new Error("Persist not initialized");
		}

		// Find insertion point (events are sorted by timestamp)
		const insertIndex = this.#persist.scheduledEvents.findIndex(
			(x: PersistedScheduleEvent) => x.timestamp > newEvent.timestamp,
		);

		if (insertIndex === -1) {
			// Add to end
			this.#persist.scheduledEvents.push(newEvent);
		} else {
			// Insert at correct position
			this.#persist.scheduledEvents.splice(insertIndex, 0, newEvent);
		}

		// Update alarm if this is the newest event
		if (insertIndex === 0 || this.#persist.scheduledEvents.length === 1) {
			this.#actor.log.info({
				msg: "setting alarm for new event",
				timestamp: newEvent.timestamp,
				eventCount: this.#persist.scheduledEvents.length,
			});
			await this.#queueSetAlarm(newEvent.timestamp);
		}
	}

	async #executeDueEvents(events: PersistedScheduleEvent[]): Promise<void> {
		for (const event of events) {
			const span = this.#actor.startTraceSpan(
				`actor.action.${event.action}`,
				{
					"rivet.action.name": event.action,
					"rivet.action.scheduled": true,
					"rivet.schedule.event_id": event.eventId,
					"rivet.schedule.timestamp_ms": event.timestamp,
				},
			);
			try {
				this.#actor.emitTraceEvent(
					"schedule.triggered",
					{
						"rivet.schedule.event_id": event.eventId,
						"rivet.schedule.action": event.action,
						"rivet.schedule.timestamp_ms": event.timestamp,
					},
					span,
				);
				this.#actor.log.info({
					msg: "executing scheduled event",
					eventId: event.eventId,
					timestamp: event.timestamp,
					action: event.action,
				});

				// Look up the action function
				const actions = this.#config.actions ?? {};
				const fn = actions[event.action];

				if (!fn) {
					throw new Error(
						`Missing action for scheduled event: ${event.action}`,
					);
				}

				if (typeof fn !== "function") {
					throw new Error(
						`Scheduled event action ${event.action} is not a function (got ${typeof fn})`,
					);
				}

				// Decode arguments and execute
				const args = event.args
					? cbor.decode(new Uint8Array(event.args))
					: [];

				const result = this.#actor.traces.withSpan(span, () =>
					fn.call(undefined, this.#actor.actorContext, ...args),
				);

				// Handle async actions
				if (result instanceof Promise) {
					await result;
				}

				this.#actor.endTraceSpan(span, { code: "OK" });
				this.#actor.log.debug({
					msg: "scheduled event completed",
					eventId: event.eventId,
					action: event.action,
				});
			} catch (error) {
				this.#actor.traces.setAttributes(span, {
					"error.message": stringifyError(error),
					"error.type":
						error instanceof Error ? error.name : typeof error,
				});
				this.#actor.endTraceSpan(span, {
					code: "ERROR",
					message: stringifyError(error),
				});
				this.#actor.log.error({
					msg: "error executing scheduled event",
					error: stringifyError(error),
					eventId: event.eventId,
					timestamp: event.timestamp,
					action: event.action,
				});

				// Continue processing other events even if one fails
			}
		}
	}

	async #queueSetAlarm(timestamp: number): Promise<void> {
		await this.#alarmWriteQueue.enqueue(async () => {
			await this.#actorDriver.setAlarm(this.#actor, timestamp);
		});
	}

	/**
	 * Gets the next scheduled event, if any.
	 */
	getNextEvent(): PersistedScheduleEvent | null {
		if (
			!this.#persist?.scheduledEvents ||
			this.#persist.scheduledEvents.length === 0
		) {
			return null;
		}
		return this.#persist.scheduledEvents[0];
	}

	/**
	 * Gets all scheduled events.
	 */
	getAllEvents(): PersistedScheduleEvent[] {
		return this.#persist?.scheduledEvents || [];
	}

	/**
	 * Clears all scheduled events.
	 * Use with caution - this removes all pending scheduled events.
	 */
	clearAllEvents(): void {
		if (this.#persist?.scheduledEvents) {
			this.#persist.scheduledEvents = [];
			this.#actor.log.warn({ msg: "cleared all scheduled events" });
		}
	}
}
