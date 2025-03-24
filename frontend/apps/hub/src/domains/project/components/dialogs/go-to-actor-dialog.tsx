import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@rivet-gg/components";
import * as GoToActorForm from "@/domains/project/forms/go-to-actor-form";
import { useNavigate } from "@tanstack/react-router";

interface ContentProps extends DialogContentProps {
	projectNameId: string;
	environmentNameId: string;
}

export default function GoToActorDialogContent({
	projectNameId,
	environmentNameId,
	onClose,
}: ContentProps) {
	const navigate = useNavigate();
	return (
		<GoToActorForm.Form
			defaultValues={{ actorId: "" }}
			onSubmit={({ actorId }) => {
				navigate({
					to: "/projects/$projectNameId/environments/$environmentNameId/actors",
					params: {
						projectNameId,
						environmentNameId,
					},
					search: {
						actorId,
						modal: undefined,
					},
				});
			}}
		>
			<DialogHeader>
				<DialogTitle>Go to Actor</DialogTitle>
			</DialogHeader>
			<GoToActorForm.ActorId />
			<DialogFooter>
				<GoToActorForm.Submit>Go</GoToActorForm.Submit>
				<Button type="button" variant="secondary" onClick={onClose}>
					Close
				</Button>
			</DialogFooter>
		</GoToActorForm.Form>
	);
}
