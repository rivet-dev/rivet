import { useOrganization } from "@clerk/clerk-react";
import type { Rivet } from "@rivet-gg/cloud";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";

const useDefaultOrg = () => {
	if (__APP_TYPE__ === "cloud") {
		// biome-ignore lint/correctness/useHookAtTopLevel: secured by build condition
		const user = useOrganization();

		return user.organization?.id;
	}

	return undefined;
};

export default function CreateProjectFrameContent({
	organization,
	onSuccess,
	name,
}: {
	name?: string;
	organization?: string;
	// FIXME
	onSuccess?: (
		data: Rivet.ProjectsCreateResponse,
		vars: { displayName: string; organization: string },
	) => void;
}) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const params = useParams({ strict: false });

	const provider = useCloudDataProvider();

	const defaultOrg = useDefaultOrg();

	const { mutateAsync } = useMutation({
		...provider.createProjectMutationOptions(),
		onSuccess: async (data, vars) => {
			if (!params.organization) {
				return;
			}

			await queryClient.invalidateQueries(
				provider.currentOrgProjectsQueryOptions(),
			);

			return onSuccess
				? onSuccess(data, vars)
				: navigate({
						to: "/orgs/$organization/projects/$project",
						params: {
							organization: params.organization,
							project: data.project.name,
						},
					});
		},
	});

	return (
		<CreateProjectForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
					organization: values.organization,
				});
			}}
			defaultValues={{
				name: name,
				organization: organization ?? defaultOrg ?? "",
			}}
		>
			<Frame.Header>
				<Frame.Title>Create Project</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateProjectForm.Organization />
					<CreateProjectForm.Name />
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
