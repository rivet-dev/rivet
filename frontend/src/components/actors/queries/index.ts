import type { Rivet } from "@rivetkit/engine-api-full";

export type ActorId = string;

export type ActorStatus =
	| "starting"
	| "running"
	| "stopped"
	| "crashed"
	| "sleeping"
	| "pending"
	| "crash-loop"
	| "unknown";

export function getActorStatus(
	actor: Pick<
		Rivet.Actor,
		| "createTs"
		| "destroyTs"
		| "sleepTs"
		| "pendingAllocationTs"
		| "rescheduleTs"
		| "connectableTs"
	>,
): ActorStatus {
	const {
		createTs,
		connectableTs,
		destroyTs,
		sleepTs,
		pendingAllocationTs,
		rescheduleTs,
	} = actor;

	if (rescheduleTs) {
		return "crash-loop";
	}

	if (pendingAllocationTs && !connectableTs && !destroyTs) {
		return "pending";
	}

	if (createTs && sleepTs && !destroyTs) {
		return "sleeping";
	}

	if (createTs && !connectableTs && !destroyTs) {
		return "starting";
	}

	if (createTs && connectableTs && !destroyTs) {
		return "running";
	}

	if (createTs && connectableTs && destroyTs) {
		return "stopped";
	}

	if (createTs && !connectableTs && destroyTs) {
		return "crashed";
	}

	return "unknown";
}
