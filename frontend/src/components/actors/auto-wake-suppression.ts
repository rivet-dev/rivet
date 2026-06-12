import { useSyncExternalStore } from "react";
import type { ActorId } from "./queries";

// How the dashboard should hold off auto-waking an actor the user just acted on.
//
// - "sleep": the user wants the actor to stay asleep. The inspector connection
//   is dropped too (its /metadata polling and websocket count as activity and
//   keep the actor awake), and /health auto-wake is suppressed.
// - "reschedule": the actor reallocates back to running on its own. Only the
//   /health auto-wake is suppressed so the frontend does not race the
//   reallocation; the inspector stays connected so it cannot get stuck if the
//   sleep blip during reallocation is too brief to observe.
//
// In both cases suppression is cleared once the actor transitions into running
// again (see `ActorsActorDetails` in actor-details-iframe.tsx), so normal
// auto-wake resumes. The store is in-memory, so a full page reload clears it.
export type AutoWakeSuppression = "sleep" | "reschedule";

const suppressed = new Map<ActorId, AutoWakeSuppression>();
const listeners = new Set<() => void>();

function emit() {
	for (const listener of listeners) {
		listener();
	}
}

export function suppressAutoWake(actorId: ActorId, mode: AutoWakeSuppression) {
	suppressed.set(actorId, mode);
	emit();
}

export function resumeAutoWake(actorId: ActorId) {
	if (suppressed.delete(actorId)) {
		emit();
	}
}

export function useAutoWakeSuppression(
	actorId: ActorId,
): AutoWakeSuppression | undefined {
	return useSyncExternalStore(
		(onChange) => {
			listeners.add(onChange);
			return () => listeners.delete(onChange);
		},
		() => suppressed.get(actorId),
	);
}
