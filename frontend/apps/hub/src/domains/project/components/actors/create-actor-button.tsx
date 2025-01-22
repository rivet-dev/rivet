import { Button, type ButtonProps } from "@rivet-gg/components";
import { Icon, faPlus } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";

export function CreateActorButton(props: ButtonProps) {
	const navigate = useNavigate();
	return (
		<Button
			size="sm"
			variant="ghost"
			onClick={() => {
				navigate({
					to: ".",
					search: (prev) => ({ ...prev, modal: "create-actor" }),
				});
			}}
			startIcon={<Icon icon={faPlus} />}
			{...props}
		>
			Create Actor
		</Button>
	);
}
