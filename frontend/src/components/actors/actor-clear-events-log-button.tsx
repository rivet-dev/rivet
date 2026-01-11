import { faBroomWide, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { Button } from "../ui/button";
import { WithTooltip } from "../ui/tooltip";
import { useActorInspector } from "./actor-inspector-context";
import type { ActorId } from "./queries";

export function ActorClearEventsLogButton({ actorId }: { actorId: ActorId }) {
	const { mutate, isPending } = useMutation(
		useActorInspector().actorClearEventsMutationOptions(actorId),
	);

	return (
		<WithTooltip
			content="Clear events log"
			trigger={
				<Button
					isLoading={isPending}
					variant="outline"
					size="icon-sm"
					onClick={() => {
						mutate();
					}}
				>
					<Icon icon={faBroomWide} />
				</Button>
			}
		/>
	);
}
