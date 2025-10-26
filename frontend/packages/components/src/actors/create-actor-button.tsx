import { Button, type ButtonProps, WithTooltip } from "@rivet-gg/components";
import { faPlus, Icon } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { useAtomValue } from "jotai";
import {
	actorBuildsCountAtom,
	actorManagerEndpointAtom,
} from "./actor-context";
import { useActorsView } from "./actors-view-context-provider";

export function CreateActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	const builds = useAtomValue(actorBuildsCountAtom);
	const endpoint = useAtomValue(actorManagerEndpointAtom);

	const { copy, canCreate: contextAllowActorsCreation } = useActorsView();

	const canCreate = builds > 0 && contextAllowActorsCreation && endpoint;

	if (!contextAllowActorsCreation) {
		return null;
	}

	const content = (
		<div>
			<Button
				disabled={!canCreate}
				size="sm"
				variant="ghost"
				onClick={() => {
					navigate({
						to: ".",
						search: (prev) => ({
							...prev,
							modal: "create-actor",
						}),
					});
				}}
				startIcon={<Icon icon={faPlus} />}
				{...props}
			>
				{copy.createActor}
			</Button>
		</div>
	);

	if (canCreate) {
		return content;
	}

	return (
		<WithTooltip
			trigger={content}
			content={
				builds <= 0 || !endpoint
					? "Please deploy a build first."
					: copy.createActorUsingForm
			}
		/>
	);
}
