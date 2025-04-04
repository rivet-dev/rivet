import { Button, WithTooltip, type ButtonProps } from "@rivet-gg/components";
import { Icon, faPlus } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { useAtomValue } from "jotai";
import {
	actorBuildsCountAtom,
	actorManagerEndpointAtom,
} from "./actor-context";

export function CreateActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	const endpoint = useAtomValue(actorManagerEndpointAtom);
	const builds = useAtomValue(actorBuildsCountAtom);

	const canCreate = builds > 0 && endpoint;

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
						Create Actor
					</Button>
				</div>
			}
			content={
				builds <= 0
					? "No builds found, please deploy a build first."
					: endpoint
						? "Create new Actor using simple form."
						: "No Actor Manager found, please deploy a build first."
			}
		/>
	);
}
