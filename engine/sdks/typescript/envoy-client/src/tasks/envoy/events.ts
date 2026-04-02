import type * as protocol from "@rivetkit/engine-envoy-protocol";
import type { EnvoyContext, ToEnvoyMessage } from "./index.js";
import { getActorEntry, log } from "./index.js";
import { wsSend } from "../connection.js";

export function handleSendEvents(
	ctx: EnvoyContext,
	events: protocol.EventWrapper[],
): boolean {
	let stop = false;

	// Record in history per actor
	for (const event of events) {
		const entry = getActorEntry(
			ctx,
			event.checkpoint.actorId,
			event.checkpoint.generation,
		);
		if (entry) {
			entry.eventHistory.push(event);

			// Close the actor channel but keep event history for ack/resend.
			// The entry is cleaned up when all events are acked.
			if (event.inner.tag === "EventActorStateUpdate") {
				if (event.inner.val.state.tag === "ActorStateStopped") {
					entry.handle.close();

					// Serverless envoys only handle one actor which means if it stops, the envoy should stop too
					if (ctx.serverless) stop = true;
				}
			}
		}
	}

	// Send if connected
	wsSend(ctx.shared, {
		tag: "ToRivetEvents",
		val: events,
	});

	return stop;
}

export function handleAckEvents(
	ctx: EnvoyContext,
	ack: protocol.ToEnvoyAckEvents,
) {
	for (const checkpoint of ack.lastEventCheckpoints) {
		const entry = getActorEntry(
			ctx,
			checkpoint.actorId,
			checkpoint.generation,
		);
		if (!entry) continue;

		entry.eventHistory = entry.eventHistory.filter(
			(event) => event.checkpoint.index > checkpoint.index,
		);

		// Clean up fully acked stopped actors
		if (entry.eventHistory.length === 0 && entry.handle.isClosed()) {
			const gens = ctx.actors.get(checkpoint.actorId);
			gens?.delete(checkpoint.generation);
			if (gens?.size === 0) {
				ctx.actors.delete(checkpoint.actorId);
			}
		}
	}
}

export function resendUnacknowledgedEvents(ctx: EnvoyContext) {
	const events: protocol.EventWrapper[] = [];

	for (const [, generations] of ctx.actors) {
		for (const [, entry] of generations) {
			events.push(...entry.eventHistory);
		}
	}

	if (events.length === 0) return;

	log(ctx.shared)?.info({
		msg: "resending unacknowledged events",
		count: events.length,
	});

	wsSend(ctx.shared, {
		tag: "ToRivetEvents",
		val: events,
	});
}
