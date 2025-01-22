import { Button, WithTooltip } from "@rivet-gg/components";
import { Icon, faXmark } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import {
	actorDestroyedAtQueryOptions,
	useDestroyActorMutation,
} from "../../queries";

interface ActorStopButtonProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}

export function ActorStopButton({
	projectNameId,
	environmentNameId,
	actorId,
}: ActorStopButtonProps) {
	const { data: destroyedAt } = useSuspenseQuery(
		actorDestroyedAtQueryOptions({
			projectNameId,
			environmentNameId,
			actorId,
		}),
	);
	const { mutate, isPending: isDestroying } = useDestroyActorMutation();

	if (destroyedAt) {
		return null;
	}

	return (
		<WithTooltip
			trigger={
				<Button
					isLoading={isDestroying}
					variant="destructive"
					size="icon-sm"
					onClick={() =>
						mutate({
							projectNameId,
							environmentNameId,
							actorId,
						})
					}
				>
					<Icon icon={faXmark} />
				</Button>
			}
			content="Stop Actor"
		/>
	);
}
