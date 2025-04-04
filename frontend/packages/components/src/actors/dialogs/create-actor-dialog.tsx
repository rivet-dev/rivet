import * as ActorCreateForm from "../form/actor-create-form";
import type { DialogContentProps } from "../../hooks";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui/dialog";
import { Flex } from "../../ui/flex";
import { useAtomValue } from "jotai";
import { createActorAtom } from "../actor-context";

interface ContentProps extends DialogContentProps {}

export default function CreateActorDialog({ onClose }: ContentProps) {
	const { endpoint, create } = useAtomValue(createActorAtom);

	return (
		<>
			<ActorCreateForm.Form
				onSubmit={async (values) => {
					if (!endpoint) {
						throw new Error("No endpoint");
					}
					await create({
						endpoint,
						name: values.buildId,
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
					<DialogTitle>Create Actor</DialogTitle>
					<DialogDescription>
						Choose a build to create an Actor from. Actor will be
						created using default settings.
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
