import { faBroomWide, Icon } from "@rivet-gg/icons";
import { Button } from "../ui/button";
import { WithTooltip } from "../ui/tooltip";
import { type ActorId, useActorClearEventsMutation } from "./queries";

export function ActorClearEventsLogButton({ actorId }: { actorId: ActorId }) {
	const { mutate, isPending } = useActorClearEventsMutation(actorId);

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
