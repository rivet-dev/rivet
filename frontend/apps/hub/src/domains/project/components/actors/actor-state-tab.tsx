import { Code, DocsSheet } from "@rivet-gg/components";
import { Icon, faWarning } from "@rivet-gg/icons";
import { ActorEditableState } from "./actor-editable-state";
import {
	useActorState,
	useActorWorkerStatus,
} from "./worker/actor-worker-context";

interface ActorStateTabProps {
	disabled?: boolean;
}

export function ActorStateTab({ disabled }: ActorStateTabProps) {
	const status = useActorWorkerStatus();

	const state = useActorState();

	if (disabled) {
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

	if (!state.enabled && status.type === "ready") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center flex-col">
				State is not enabled for this actor.
				<DocsSheet
					title="State"
					path="docs/state"
					hash="initializing-and-updating-state"
				>
					<span className="hover:underline cursor-pointer">
						Enable it by adding{" "}
						<Code className="text-xs">_onInitialize</Code> method.
					</span>
				</DocsSheet>
			</div>
		);
	}
	return (
		<div className="flex-1 w-full min-h-0 h-full flex flex-col">
			<ActorEditableState state={state} />
		</div>
	);
}
