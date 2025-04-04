import { ActorEditableState } from "./actor-editable-state";
import {
	useActorState,
	useActorWorkerStatus,
} from "./worker/actor-worker-context";
import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";

const selector = (a: Actor) => a.destroyedAt;

interface ActorStateTabProps {
	actor: ActorAtom;
}

export function ActorStateTab({ actor }: ActorStateTabProps) {
	const destroyedAt = useAtomValue(selectAtom(actor, selector));
	const status = useActorWorkerStatus();

	const state = useActorState();

	if (destroyedAt) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				State Preview is unavailable for inactive Actors.
			</div>
		);
	}

	if (status.type === "error") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				State Preview is currently unavailable.
				<br />
				See console/logs for more details.
			</div>
		);
	}

	if (status.type === "unsupported") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				State Preview is not supported for this Actor.
			</div>
		);
	}

	if (status.type !== "ready") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				Loading state...
			</div>
		);
	}

	return (
		<div className="flex-1 w-full min-h-0 h-full flex flex-col">
			<ActorEditableState state={state} />
		</div>
	);
}
