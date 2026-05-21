import type { Rivet } from "@rivet-gg/cloud";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

const useDefaultOrg = () => {
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const org = authClient.useActiveOrganization();
		return org.data?.id;
	}
	return undefined;
};

export type CreateProjectSuccessVars = {
	displayName: string;
	organization: string;
};

export default function CreateProjectFrameContent({
	organization,
	onSuccess,
	name,
}: {
	name?: string;
	organization?: string;
	onSuccess?: (
		data: Rivet.ProjectsCreateResponse,
		vars: CreateProjectSuccessVars,
	) => void;
}) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const provider = useCloudDataProvider();

	const defaultOrg = useDefaultOrg();

	const { mutateAsync } = useMutation({
		...provider.createProjectMutationOptions(),
	});

	return (
		<CreateProjectForm.Form
			onSubmit={async (values) => {
				const result = await mutateAsync({
					displayName: values.name,
					organization: values.organization,
				});

				await queryClient.refetchQueries(
					provider.currentOrgProjectsQueryOptions(),
				);

				const successVars: CreateProjectSuccessVars = {
					displayName: values.name,
					organization: values.organization,
				};

				if (onSuccess) {
					onSuccess(result, successVars);
					return;
				}

				navigate({
					to: "/orgs/$organization/projects/$project",
					params: {
						organization: values.organization,
						project: result.project.name,
					},
				});
			}}
			defaultValues={{
				name: name,
				organization: organization ?? defaultOrg ?? "",
			}}
		>
			<Frame.Header>
				<Frame.Title>Create new project</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateProjectForm.Organization />
					<CreateProjectForm.Name />
				</Flex>
			</Frame.Content>
			<Frame.Footer className="flex-row justify-end">
				<CreateProjectForm.DefaultSubmit />
			</Frame.Footer>
		</CreateProjectForm.Form>
	);
}
