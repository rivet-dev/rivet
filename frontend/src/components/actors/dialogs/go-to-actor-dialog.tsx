import { Button } from "../../ui/button";
import { DialogFooter, DialogHeader, DialogTitle } from "../../ui/dialog";
import { useActorsView } from "../actors-view-context-provider";
import * as GoToActorForm from "../form/go-to-actor-form";
import type { DialogContentProps } from "../hooks";

interface ContentProps extends DialogContentProps {
	onSubmit?: (actorId: string) => void;
}

export default function GoToActorDialogContent({
	onClose,
	onSubmit,
}: ContentProps) {
	const { copy } = useActorsView();
	return (
		<GoToActorForm.Form
			defaultValues={{ actorId: "" }}
			onSubmit={({ actorId }) => {
				onSubmit?.(actorId);
			}}
		>
			<DialogHeader>
				<DialogTitle>{copy.goToActor}</DialogTitle>
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
