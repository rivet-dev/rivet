import { faMagnifyingGlass, Icon } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { Button, type ButtonProps } from "@/components";
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
