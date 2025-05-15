import { Button, WithTooltip, type ButtonProps } from "@rivet-gg/components";
import { Icon, faPlus } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { useAtomValue } from "jotai";
import { actorBuildsCountAtom } from "./actor-context";
import { useActorsView } from "./actors-view-context-provider";

export function CreateActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	const builds = useAtomValue(actorBuildsCountAtom);

	const { copy, canCreate: canCreateActors } = useActorsView();

	const canCreate = builds > 0 && canCreateActors;

	if (!canCreateActors) {
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
				builds <= 0
					? "Please deploy a build first."
					: copy.createActorUsingForm
			}
		/>
	);
}
