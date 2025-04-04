import * as GoToActorForm from "../form/go-to-actor-form";
import type { DialogContentProps } from "../../hooks";
import { DialogFooter, DialogHeader, DialogTitle } from "../../ui/dialog";
import { Button } from "../../ui/button";

interface ContentProps extends DialogContentProps {
	onSubmit?: (actorId: string) => void;
}

export default function GoToActorDialogContent({
	onClose,
	onSubmit,
}: ContentProps) {
	return (
		<GoToActorForm.Form
			defaultValues={{ actorId: "" }}
			onSubmit={({ actorId }) => {
				onSubmit?.(actorId);
			}}
		>
			<DialogHeader>
				<DialogTitle>Go to Actor</DialogTitle>
			</DialogHeader>
			<GoToActorForm.ActorId />
			<DialogFooter>
				<Button type="button" variant="secondary" onClick={onClose}>
					Close
				</Button>
				<GoToActorForm.Submit>Go</GoToActorForm.Submit>
			</DialogFooter>
		</GoToActorForm.Form>
	);
}
