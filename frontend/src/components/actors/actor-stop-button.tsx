import { faMoon, faRefresh, faTrash, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Button } from "@/components";
import { queryClient } from "@/queries/global";
import { suppressAutoWake } from "./auto-wake-suppression";
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
			variant="destructive-outline"
			size="sm"
			startIcon={isConfirming ? undefined : <Icon icon={faTrash} />}
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
					: "Destroy"}
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
		<Button
			isLoading={isPending}
			variant="outline"
			size="sm"
			startIcon={<Icon icon={faMoon} />}
			onClick={(e) => {
				e?.stopPropagation();
				// Keep the actor asleep. Without this the auto-wake-on-select
				// controller and the inspector connection would immediately wake
				// it back up.
				suppressAutoWake(actorId, "sleep");
				mutate();
			}}
		>
			Sleep
		</Button>
	);
}

export function ActorRescheduleButton({ actorId }: { actorId: ActorId }) {
	const dataProvider = useDataProvider();
	const { data: destroyedAt } = useQuery(
		dataProvider.actorDestroyedAtQueryOptions(actorId),
	);
	const { mutate, isPending } = useMutation(
		dataProvider.actorRescheduleAfterSleepMutationOptions(actorId),
	);
	const { canRescheduleActors } = dataProvider.features;

	// Reschedule is not gated on status. It reallocates the actor regardless of
	// whether it is running, sleeping, crashing, pending, etc. Only hidden once
	// the actor is destroyed, since there is nothing left to reallocate.
	if (!canRescheduleActors || destroyedAt) {
		return null;
	}

	return (
		<Button
			isLoading={isPending}
			variant="outline"
			size="sm"
			startIcon={<Icon icon={faRefresh} />}
			onClick={(e) => {
				e?.stopPropagation();
				// Reschedule tells the engine to sleep and reallocate the actor,
				// which ends up running again. Suppress frontend /health auto-wake
				// until it is running again so the reallocation is not raced. The
				// inspector stays connected and reconnects on its own.
				suppressAutoWake(actorId, "reschedule");
				mutate();
			}}
		>
			Reschedule
		</Button>
	);
}
