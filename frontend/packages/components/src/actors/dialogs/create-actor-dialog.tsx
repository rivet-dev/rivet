import * as ActorCreateForm from "../form/actor-create-form";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui/dialog";
import { Flex } from "../../ui/flex";
import { useAtomValue } from "jotai";
import { createActorAtom } from "../actor-context";
import type { DialogContentProps } from "../hooks";
import { useActorsView } from "../actors-view-context-provider";

interface ContentProps extends DialogContentProps {}

export default function CreateActorDialog({ onClose }: ContentProps) {
	const { endpoint, create } = useAtomValue(createActorAtom);

	const { copy } = useActorsView();

	return (
		<>
			<ActorCreateForm.Form
				onSubmit={async (values) => {
					if (!endpoint) {
						throw new Error("No endpoint");
					}
					await create({
						endpoint,
						id: values.buildId,
						tags: Object.fromEntries(
							values.tags.map((tag) => [tag.key, tag.value]),
						),
						params: values.parameters
							? JSON.parse(values.parameters)
							: undefined,
						region: values.regionId,
					});
					onClose?.();
				}}
				defaultValues={{ buildId: "", regionId: "" }}
			>
				<DialogHeader>
					<DialogTitle>{copy.createActorModal.title}</DialogTitle>
					<DialogDescription>
						{copy.createActorModal.description}
					</DialogDescription>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<ActorCreateForm.Build />
					<ActorCreateForm.Region />
					<ActorCreateForm.Tags />
					{/* <ActorCreateForm.Parameters /> */}
				</Flex>
				<DialogFooter>
					<ActorCreateForm.Submit type="submit">
						Create
					</ActorCreateForm.Submit>
				</DialogFooter>
			</ActorCreateForm.Form>
		</>
	);
}
