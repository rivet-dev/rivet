import * as ProjectCreateForm from "@/domains/project/forms/create-project-form";
import { convertStringToId } from "@/lib/utils";
import type { Rivet } from "@rivet-gg/api-full";
import {
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
} from "@rivet-gg/components";
import { useProjectCreateMutation } from "../../queries";

interface CreateProjectDialogContentProps {
	groupId?: string;
	onSuccess?: (data: Rivet.cloud.GameFull) => void;
}

export default function CreateProjectDialogContent({
	onSuccess,
	groupId = "",
}: CreateProjectDialogContentProps) {
	const { mutateAsync } = useProjectCreateMutation({
		onSuccess,
	});

	return (
		<>
			<ProjectCreateForm.Form
				onSubmit={async ({ name, slug, developerGroupId }) => {
					await mutateAsync({
						developerGroupId,
						displayName: name,
						nameId: slug || convertStringToId(name),
					});
				}}
				defaultValues={{
					name: "",
					slug: "",
					developerGroupId: groupId,
				}}
			>
				<DialogHeader>
					<DialogTitle>Create New Project</DialogTitle>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<ProjectCreateForm.Group />
					<ProjectCreateForm.Name />
					<ProjectCreateForm.Slug />
				</Flex>
				<DialogFooter>
					<ProjectCreateForm.Submit type="submit">
						Create
					</ProjectCreateForm.Submit>
				</DialogFooter>
			</ProjectCreateForm.Form>
		</>
	);
}
