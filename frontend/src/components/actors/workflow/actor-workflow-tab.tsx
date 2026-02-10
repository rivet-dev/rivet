import { faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { useActorInspector } from "../actor-inspector-context";
import type { ActorId } from "../queries";
import { WorkflowVisualizer } from "./workflow-visualizer";

interface ActorWorkflowTabProps {
	actorId: ActorId;
}

export function ActorWorkflowTab({ actorId }: ActorWorkflowTabProps) {
	const inspector = useActorInspector();

	const { data: isWorkflowEnabled, isLoading: isEnabledLoading } = useQuery(
		inspector.actorIsWorkflowEnabledQueryOptions(actorId),
	);

	const { data: workflowData, isLoading: isHistoryLoading } = useQuery(
		inspector.actorWorkflowHistoryQueryOptions(actorId),
	);

	const isLoading = isEnabledLoading || isHistoryLoading;
	const workflow = workflowData?.history ?? null;

	if (isLoading) {
		return (
			<Info>
				<div className="flex items-center">
					<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
					Loading Workflow...
				</div>
			</Info>
		);
	}

	if (!isWorkflowEnabled) {
		return (
			<Info>
				<p>
					Workflow Visualizer is not enabled for this Actor. <br />{" "}
					This feature requires a workflow-based Actor.
				</p>
			</Info>
		);
	}

	if (!workflow) {
		return (
			<Info>
				<p>No workflow history available yet.</p>
			</Info>
		);
	}

	return (
		<div className="flex-1 w-full min-h-0 h-full flex flex-col">
			<WorkflowVisualizer workflow={workflow} />
		</div>
	);
}

function Info({ children }: PropsWithChildren) {
	return (
		<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center max-w-lg mx-auto">
			{children}
		</div>
	);
}
