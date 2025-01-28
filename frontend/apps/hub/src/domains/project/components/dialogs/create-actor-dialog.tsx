import * as ActorCreateForm from "@/domains/project/forms/actor-create-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
} from "@rivet-gg/components";
import { useCreateActorFromSdkMutation } from "../../queries";

interface ContentProps extends DialogContentProps {
	projectNameId: string;
	environmentNameId: string;
}

export default function CreateActorDialog({
	onClose,
	projectNameId,
	environmentNameId,
}: ContentProps) {
	const { mutateAsync } = useCreateActorFromSdkMutation({
		onSuccess: onClose,
	});

	return (
		<>
			<ActorCreateForm.Form
				onSubmit={async (values) => {
					await mutateAsync({
						projectNameId,
						environmentNameId,
						buildId: values.buildId,
						region: values.regionId,
						parameters: values.parameters,
					});
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
					<ActorCreateForm.Build
						projectNameId={projectNameId}
						environmentNameId={environmentNameId}
					/>
					<ActorCreateForm.Region
						projectNameId={projectNameId}
						environmentNameId={environmentNameId}
					/>
					<ActorCreateForm.Tags
						projectNameId={projectNameId}
						environmentNameId={environmentNameId}
					/>
					<ActorCreateForm.Parameters />
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
