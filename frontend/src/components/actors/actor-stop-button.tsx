import { faMoon, faRefresh, faXmark, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Button, WithTooltip } from "@/components";
import { queryClient } from "@/queries/global";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorStopButtonProps {
	actorId: ActorId;
}

export function ActorStopButton({ actorId }: ActorStopButtonProps) {
	const provider = useDataProvider();
	const { data: destroyedAt } = useQuery(
		provider.actorDestroyedAtQueryOptions(actorId),
	);

	const { mutate, isPending } = useMutation({
		...provider.actorDestroyMutationOptions(actorId),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				provider.actorQueryOptions(actorId),
			);
		},
	});

	const { canDeleteActors } = provider.features;
	const [isConfirming, setIsConfirming] = useState(false);

	useEffect(() => {
		if (isConfirming) {
			const timer = setTimeout(() => {
				setIsConfirming(false);
			}, 4000);

			return () => clearTimeout(timer);
		}
	}, [isConfirming]);

	if (!canDeleteActors) {
		return null;
	}

	if (destroyedAt) {
		return null;
	}

	return (
		<Button
			isLoading={isPending}
			variant="destructive"
			size="sm"
			onClick={(e) => {
				e?.stopPropagation();
				if (e?.shiftKey || isConfirming) {
					mutate();
					return;
				}

				setIsConfirming(true);
			}}
		>
			{isPending
				? "Destroying..."
				: isConfirming
					? "Are you sure? What's gone is gone."
					: "Destroy Actor"}
		</Button>
	);
}

export function ActorSleepButton({ actorId }: { actorId: ActorId }) {
	const dataProvider = useDataProvider();
	const { data: status } = useQuery(
		dataProvider.actorStatusQueryOptions(actorId),
	);
	const { mutate, isPending } = useMutation(
		dataProvider.actorSleepMutationOptions(actorId),
	);
	const { canSleepActors } = dataProvider.features;

	if (!canSleepActors || status !== "running") {
		return null;
	}

	return (
		<WithTooltip
			delayDuration={0}
			trigger={
				<Button
					isLoading={isPending}
					variant="outline"
					size="icon-sm"
					onClick={(e) => {
						e?.stopPropagation();
						mutate();
					}}
				>
					<Icon icon={faMoon} />
				</Button>
			}
			content="Sleep Actor"
		/>
	);
}

export function ActorRescheduleButton({ actorId }: { actorId: ActorId }) {
	const dataProvider = useDataProvider();
	const { data: status } = useQuery(
		dataProvider.actorStatusQueryOptions(actorId),
	);
	const { mutate, isPending } = useMutation(
		dataProvider.actorRescheduleAfterSleepMutationOptions(actorId),
	);
	const { canRescheduleActors } = dataProvider.features;

	if (!canRescheduleActors || status !== "sleeping") {
		return null;
	}

	return (
		<WithTooltip
			delayDuration={0}
			trigger={
				<Button
					isLoading={isPending}
					variant="outline"
					size="icon-sm"
					onClick={(e) => {
						e?.stopPropagation();
						mutate();
					}}
				>
					<Icon icon={faRefresh} />
				</Button>
			}
			content="Reschedule Actor"
		/>
	);
}
