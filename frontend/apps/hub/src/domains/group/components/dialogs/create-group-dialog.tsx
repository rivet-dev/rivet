import * as GroupCreateForm from "@/domains/group/forms/group-create-form";
import type { Rivet } from "@rivet-gg/api";
import {
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
} from "@rivet-gg/components";
import { useGroupCreateMutation } from "../../queries";

interface CreateGroupDialogContentProps {
	onSuccess?: (data: Rivet.group.CreateResponse) => void;
}

export default function CreateGroupDialogContent({
	onSuccess,
}: CreateGroupDialogContentProps) {
	const { mutateAsync } = useGroupCreateMutation({
		onSuccess,
	});

	return (
		<>
			<GroupCreateForm.Form
				onSubmit={async (values) => {
					await mutateAsync({
						displayName: values.name,
					});
				}}
				defaultValues={{ name: "" }}
			>
				<DialogHeader>
					<DialogTitle>Create New Team</DialogTitle>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<GroupCreateForm.Name />
				</Flex>
				<DialogFooter>
					<GroupCreateForm.Submit type="submit">
						Create
					</GroupCreateForm.Submit>
				</DialogFooter>
			</GroupCreateForm.Form>
		</>
	);
}
