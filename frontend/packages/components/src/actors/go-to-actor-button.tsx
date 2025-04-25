import { Button, type ButtonProps } from "@rivet-gg/components";
import { Icon, faMagnifyingGlass } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { useActorsView } from "./actors-view-context-provider";

export function GoToActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	const { copy } = useActorsView();
	return (
		<Button
			size="sm"
			variant="ghost"
			onClick={() => {
				navigate({
					to: ".",
					search: (prev) => ({ ...prev, modal: "go-to-actor" }),
				});
			}}
			startIcon={<Icon icon={faMagnifyingGlass} />}
			{...props}
		>
			{copy.goToActor}
		</Button>
	);
}
