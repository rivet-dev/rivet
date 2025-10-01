import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { convertStringToId } from "@/lib/utils";

export default function CreateProjectFrameContent() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const params = useParams({ strict: false });

	const provider = useCloudDataProvider();

	const { mutateAsync } = useMutation(
		provider.currentOrgCreateProjectMutationOptions({
			onSuccess: async (values) => {
				if (!params.organization) {
					return;
				}

				await queryClient.invalidateQueries(
					provider.currentOrgProjectsQueryOptions(),
				);

				await navigate({
					to: "/orgs/$organization/projects/$project",
					params: {
						organization: params.organization,
						project: values.project.name,
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
			<Frame.Header>
				<Frame.Title>Create Project</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateProjectForm.Name />
					<CreateProjectForm.Slug />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateProjectForm.Submit type="submit">
					Create
				</CreateProjectForm.Submit>
			</Frame.Footer>
		</CreateProjectForm.Form>
	);
}
