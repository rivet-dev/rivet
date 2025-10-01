import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { DialogFooter, DialogHeader, DialogTitle, Flex } from "@/components";
import { convertStringToId } from "@/lib/utils";
import {
	createProjectMutationOptions,
	projectsQueryOptions,
} from "@/queries/manager-cloud";
import {
	managerClient,
	namespacesQueryOptions,
} from "@/queries/manager-engine";

export default function CreateProjectDialogContent() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const { mutateAsync } = useMutation(
		createProjectMutationOptions({
			onSuccess: async (values) => {
				await queryClient.invalidateQueries({
					...projectsQueryOptions(),
				});
				navigate({
					to: "/orgs/$organization/projects/$project",
					params: {
						organization: values.organizationId,
						project: values.name,
					},
				});
			},
		}),
	);

	return (
		<CreateProjectForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
					nameId: values.slug || convertStringToId(values.name),
				});
			}}
			defaultValues={{ name: "", slug: "" }}
		>
			<DialogHeader>
				<DialogTitle>Create New Project</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<CreateProjectForm.Name />
				<CreateProjectForm.Slug />
			</Flex>
			<DialogFooter>
				<CreateProjectForm.Submit type="submit">
					Create
				</CreateProjectForm.Submit>
			</DialogFooter>
		</CreateProjectForm.Form>
	);
}
