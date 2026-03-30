import * as protocol from "@rivetkit/engine-envoy-protocol";
import {
	type UnboundedReceiver,
	type UnboundedSender,
	unboundedChannel,
} from "antiox/sync/mpsc";
import { spawn } from "antiox/task";
import type { SharedContext } from "../context.js";
import { logger } from "../log.js";
import { unreachable } from "antiox/panic";
import { promiseWithResolvers, stringifyError } from "../utils.js";

export interface CreateActorOpts {
	commandIdx: bigint;
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;
	hibernatingRequests: readonly protocol.HibernatingRequest[];
}

/**
 *
 * Stop sequence:
 * 1. X -> Actor: stop-intent (optional)
 * 1. Actor -> Envoy: send-events (optional)
 * 1. Envoy -> Actor: command-stop-actor
 * 1. Actor: async cleanup
 * 1. Actor -> Envoy: state update (stopped)
 */

// TODO: envoy lost
export type ToActor =
	// Sent when wants to stop the actor, will be forwarded to Envoy
	| {
			type: "actor-intent";
			commandIdx: bigint;
			intent: protocol.ActorIntent;
	  }
	// Sent when actor is told to stop
	| {
			type: "command-stop-actor";
			commandIdx: bigint;
			reason: protocol.StopActorReason;
	  }
	// Set or clear an alarm
	| {
			type: "set-alarm";
			alarmTs: bigint | null;
	  };

interface ActorContext {
	shared: SharedContext;
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;
	eventIndex: bigint;
}

export function createActor(
	ctx: SharedContext,
	start: CreateActorOpts,
): { tx: UnboundedSender<ToActor>; actorStartPromise: Promise<void> } {
	const [tx, rx] = unboundedChannel<ToActor>();
	const startPromise = promiseWithResolvers<void>();
	// Prevent unhandled rejection if no tunnel handler awaits this
	startPromise.promise.catch(() => {});
	spawn(() =>
		actorInner(ctx, start, rx, startPromise.resolve, startPromise.reject),
	);
	return { tx, actorStartPromise: startPromise.promise };
}

async function actorInner(
	shared: SharedContext,
	opts: CreateActorOpts,
	rx: UnboundedReceiver<ToActor>,
	resolveStart: (value: void) => void,
	rejectStart: (reason?: any) => void,
) {
	const ctx: ActorContext = {
		shared,
		actorId: opts.actorId,
		generation: opts.generation,
		config: opts.config,
		eventIndex: 0n,
	};

	let stopCode = protocol.StopCode.Ok;
	let stopMessage: string | null = null;

	try {
		await shared.config.onActorStart(
			shared.handle,
			opts.actorId,
			opts.generation,
			opts.config,
		);
	} catch (error) {
		rejectStart(
			error instanceof Error ? error : new Error("actor start failed"),
		);

		log(ctx)?.error({
			msg: "actor start failed",
			actorId: opts.actorId,
			error: stringifyError(error),
		});

		stopCode = protocol.StopCode.Error;
		stopMessage =
			error instanceof Error ? error.message : "actor start failed";

		sendStoppedEvent(ctx, stopCode, stopMessage);
		return;
	}

	resolveStart();

	sendEvent(ctx, {
		tag: "EventActorStateUpdate",
		val: { state: { tag: "ActorStateRunning", val: null } },
	});

	for await (const msg of rx) {
		if (msg.type === "actor-intent") {
			sendEvent(ctx, {
				tag: "EventActorIntent",
				val: { intent: msg.intent },
			});
		} else if (msg.type === "command-stop-actor") {
			try {
				await ctx.shared.config.onActorStop(
					ctx.shared.handle,
					ctx.actorId,
					ctx.generation,
					msg.reason,
				);
			} catch (error) {
				log(ctx)?.error({
					msg: "actor stop failed",
					actorId: ctx.actorId,
					error: stringifyError(error),
				});

				stopCode = protocol.StopCode.Error;
				stopMessage =
					error instanceof Error
						? error.message
						: "actor stop failed";
			}

			sendStoppedEvent(ctx, stopCode, stopMessage);
			return;
		} else if (msg.type === "set-alarm") {
			sendEvent(ctx, {
				tag: "EventActorSetAlarm",
				val: { alarmTs: msg.alarmTs },
			});
		} else {
			unreachable(msg);
		}
	}
}

function sendEvent(ctx: ActorContext, inner: protocol.Event) {
	ctx.shared.envoyTx.send({
		type: "send-events",
		events: [
			{
				checkpoint: incrementCheckpoint(ctx),
				inner,
			},
		],
	});
}

function sendStoppedEvent(
	ctx: ActorContext,
	code: protocol.StopCode,
	message: string | null,
) {
	const checkpoint = incrementCheckpoint(ctx);
	ctx.shared.envoyTx.send({
		type: "command-stop-actor-complete",
		checkpointIndex: checkpoint.index,
		actorId: ctx.actorId,
		generation: ctx.generation,
		code,
		message,
	});
}

function incrementCheckpoint(ctx: ActorContext): protocol.ActorCheckpoint {
	const index = ctx.eventIndex;
	ctx.eventIndex++;

	return { actorId: ctx.actorId, generation: ctx.generation, index };
}

function log(ctx: ActorContext) {
	const baseLogger = ctx.shared.config.logger ?? logger();
	if (!baseLogger) return undefined;

	return baseLogger.child({
		actorId: ctx.actorId,
		generation: ctx.generation,
	});
}
