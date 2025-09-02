import { faXmark, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Button, WithTooltip } from "@/components";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

interface ActorStopButtonProps {
	actorId: ActorId;
}

export function ActorStopButton({ actorId }: ActorStopButtonProps) {
	const { data: destroyedAt } = useQuery(
		useManager().actorDestroyedAtQueryOptions(actorId),
	);

	const { mutate, isPending } = useMutation(
		useManager().actorDestroyMutationOptions(actorId),
	);

	const { canDeleteActors } = useManager().features;
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
		<WithTooltip
			trigger={
				<Button
					isLoading={isPending}
					variant="destructive"
					size={isConfirming && !isPending ? "sm" : "icon-sm"}
					onClick={(e) => {
						e?.stopPropagation();
						if (e?.shiftKey || isConfirming) {
							mutate();
							return;
						}

						setIsConfirming(true);
					}}
				>
					{isConfirming && !isPending ? (
						"Are you sure?"
					) : (
						<Icon icon={faXmark} />
					)}
				</Button>
			}
			content="Stop Actor"
		/>
	);
}
