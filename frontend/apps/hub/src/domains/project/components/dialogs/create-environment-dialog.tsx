import * as EnvironmentCreateForm from "@/domains/project/forms/environment-create-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import { convertStringToId } from "@/lib/utils";
import {
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
} from "@rivet-gg/components";
import { useQueryClient, useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
	projectEnvironmentQueryOptions,
	projectQueryOptions,
	useEnvironmentCreateMutation,
} from "../../queries";

interface ContentProps extends DialogContentProps {
	projectId: string;
}

export default function CreateEnvironmentDialogContent({
	projectId,
}: ContentProps) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const { data: project } = useSuspenseQuery(projectQueryOptions(projectId));
	const { mutateAsync } = useEnvironmentCreateMutation({
		onSuccess: async (data) => {
			const env = await queryClient.ensureQueryData(
				projectEnvironmentQueryOptions({
					projectId,
					environmentId: data.namespaceId,
				}),
			);
			navigate({
				to: "/projects/$projectNameId/environments/$environmentNameId",
				params: {
					projectNameId: project.nameId,
					environmentNameId: env.namespace.nameId,
				},
			});
		},
	});

	return (
		<>
			<EnvironmentCreateForm.Form
				onSubmit={async (values) => {
					await mutateAsync({
						projectId,
						displayName: values.name,
						nameId: values.slug || convertStringToId(values.name),
						versionId: project.versions[0].versionId,
					});
				}}
				defaultValues={{ name: "", slug: "", projectId }}
			>
				<DialogHeader>
					<DialogTitle>Create New Environment</DialogTitle>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<EnvironmentCreateForm.Name />
					<EnvironmentCreateForm.Slug />
				</Flex>
				<DialogFooter>
					<EnvironmentCreateForm.Submit type="submit">
						Create
					</EnvironmentCreateForm.Submit>
				</DialogFooter>
			</EnvironmentCreateForm.Form>
		</>
	);
}
