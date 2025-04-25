import { Button, WithTooltip, type ButtonProps } from "@rivet-gg/components";
import { Icon, faPlus } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { useAtomValue } from "jotai";
import {
	actorBuildsCountAtom,
	actorManagerEndpointAtom,
} from "./actor-context";
import { useActorsView } from "./actors-view-context-provider";

export function CreateActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	const endpoint = useAtomValue(actorManagerEndpointAtom);
	const builds = useAtomValue(actorBuildsCountAtom);

	const canCreate = builds > 0 && endpoint;

	const { copy, requiresManager } = useActorsView();

	return (
		<WithTooltip
			trigger={
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
			}
			content={
				builds <= 0
					? "No builds found, please deploy a build first."
					: !requiresManager
						? copy.createActorUsingForm
						: endpoint
							? copy.createActorUsingForm
							: "No Actor Manager found, please deploy a build first."
			}
		/>
	);
}
