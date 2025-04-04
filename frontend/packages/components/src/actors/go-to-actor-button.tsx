import { Button, type ButtonProps } from "@rivet-gg/components";
import { Icon, faMagnifyingGlass } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";

export function GoToActorButton(props: ButtonProps) {
	const navigate = useNavigate();
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
			Go to Actor
		</Button>
	);
}
