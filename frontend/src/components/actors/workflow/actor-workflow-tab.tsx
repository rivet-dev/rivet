import { faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, type PropsWithChildren } from "react";
import { Button, toast } from "@/components";
import { useActorInspector } from "../actor-inspector-context";
import { actorInspectorQueriesKeys } from "../actor-inspector-context";
import type { ActorId } from "../queries";
import type { HistoryItem, WorkflowHistory } from "./workflow-types";
import { WorkflowVisualizer } from "./workflow-visualizer";

interface ActorWorkflowTabProps {
	actorId: ActorId;
}

export function ActorWorkflowTab({ actorId }: ActorWorkflowTabProps) {
	const inspector = useActorInspector();
	const queryClient = useQueryClient();

	const { data: isWorkflowEnabled, isLoading: isEnabledLoading } = useQuery(
		inspector.actorIsWorkflowEnabledQueryOptions(actorId),
	);

	const { data: workflowData, isLoading: isHistoryLoading } = useQuery(
		inspector.actorWorkflowHistoryQueryOptions(actorId),
	);

	const isLoading = isEnabledLoading || isHistoryLoading;
	const workflow = workflowData?.history ?? null;
	const currentStep = useMemo(() => getCurrentStep(workflow), [workflow]);
	const rerunMutation = useMutation(
		inspector.actorWorkflowRerunMutation(actorId),
	);
	const canRerunCurrentStep =
		inspector.inspectorProtocolVersion >= 4 &&
		currentStep?.entry.status !== "running" &&
		currentStep?.entry.retryCount !== undefined &&
		currentStep.entry.retryCount > 0;
	const rerunningEntryId = rerunMutation.isPending
		? rerunMutation.variables
		: undefined;

	const handleRerun = async (entryId?: string) => {
		try {
			const result = await rerunMutation.mutateAsync(entryId);
			queryClient.setQueryData(
				actorInspectorQueriesKeys.actorWorkflowHistory(actorId),
				{
					history: result.history,
					isEnabled: result.isEnabled,
				},
			);
			queryClient.setQueryData(
				actorInspectorQueriesKeys.actorIsWorkflowEnabled(actorId),
				result.isEnabled,
			);
			toast.success(
				entryId
					? "Workflow rerun scheduled from selected step."
					: "Workflow rerun scheduled from the beginning.",
			);
		} catch (error) {
			toast.error(
				error instanceof Error
					? error.message
					: "Failed to rerun workflow step.",
			);
		}
	};

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
			<div className="flex items-center justify-between gap-4 border-b border-border px-4 py-3 text-sm">
				<p className="text-muted-foreground">
					Right-click a previous step to rerun the workflow from
					there.
				</p>
				{canRerunCurrentStep && currentStep && (
					<Button
						size="sm"
						variant="outline"
						disabled={rerunMutation.isPending}
						onClick={() => handleRerun(currentStep.entry.id)}
					>
						Rerun From Current Step
					</Button>
				)}
			</div>
			<WorkflowVisualizer
				workflow={workflow}
				currentStepId={currentStep?.entry.id}
				rerunningEntryId={rerunningEntryId}
				onRerunStep={
					inspector.inspectorProtocolVersion >= 4
						? (entryId) => {
								void handleRerun(entryId);
							}
						: undefined
				}
			/>
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

function getCurrentStep(workflow: WorkflowHistory | null): HistoryItem | null {
	if (!workflow) {
		return null;
	}

	const steps = workflow.history.filter(
		(item) => item.entry.kind.type === "step",
	);
	if (steps.length === 0) {
		return null;
	}

	const activeOrFailedSteps = steps.filter(
		(item) => item.entry.status !== "completed",
	);
	if (activeOrFailedSteps.length > 0) {
		return activeOrFailedSteps[activeOrFailedSteps.length - 1]!;
	}

	return steps[steps.length - 1]!;
}
