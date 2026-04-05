import type * as protocol from "@rivetkit/engine-envoy-protocol";
import { createActor } from "../actor.js";
import { unreachable } from "antiox/panic";
import type { EnvoyContext } from "./index.js";
import { getActorEntry, log } from "./index.js";
import { wsSend } from "../connection.js";

export function handleCommands(
	ctx: EnvoyContext,
	commands: protocol.ToEnvoyCommands,
) {
	log(ctx.shared)?.info({
		msg: "received commands",
		commandCount: commands.length,
	});

	for (const commandWrapper of commands) {
		const {
			checkpoint,
			inner: { tag, val },
		} = commandWrapper;

		if (tag === "CommandStartActor") {
			const handle = createActor(ctx.shared, {
				actorId: checkpoint.actorId,
				generation: checkpoint.generation,
				config: val.config,
				hibernatingRequests: val.hibernatingRequests,
				preloadedKv: val.preloadedKv ?? null,
			});

			let generations = ctx.actors.get(checkpoint.actorId);
			if (!generations) {
				generations = new Map();
				ctx.actors.set(checkpoint.actorId, generations);
			}
			generations.set(checkpoint.generation, {
				handle,
				name: val.config.name,
				eventHistory: [],
				lastCommandIdx: checkpoint.index,
			});
		} else if (tag === "CommandStopActor") {
			const entry = getActorEntry(
				ctx,
				checkpoint.actorId,
				checkpoint.generation,
			);

			if (!entry) {
				log(ctx.shared)?.warn({
					msg: "received stop actor command for unknown actor",
					actorId: checkpoint.actorId,
					generation: checkpoint.generation,
				});
				continue;
			}

			entry.lastCommandIdx = checkpoint.index;
			entry.handle.send({
				type: "stop",
				commandIdx: checkpoint.index,
				reason: val.reason,
			});
		} else {
			unreachable(tag);
		}
	}
}

const ACK_COMMANDS_INTERVAL_MS = 5 * 60 * 1000;
export { ACK_COMMANDS_INTERVAL_MS };

export function sendCommandAck(ctx: EnvoyContext) {
	const lastCommandCheckpoints: protocol.ActorCheckpoint[] = [];

	for (const [actorId, generations] of ctx.actors) {
		for (const [generation, entry] of generations) {
			if (entry.lastCommandIdx < 0n) continue;
			lastCommandCheckpoints.push({
				actorId,
				generation,
				index: entry.lastCommandIdx,
			});
		}
	}

	if (lastCommandCheckpoints.length === 0) return;

	wsSend(ctx.shared, {
		tag: "ToRivetAckCommands",
		val: { lastCommandCheckpoints },
	});
}
