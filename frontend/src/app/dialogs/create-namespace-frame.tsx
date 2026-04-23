import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
	useNavigate,
	useParams,
	useRouteContext,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import * as CreateNamespaceForm from "@/app/forms/create-namespace-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { features } from "@/lib/features";

const useCreateNamespace = ({ project: projectProp }: { project?: string }) => {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const params = useParams({ strict: false });

	if (features.multitenancy) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const orgDataProvider = useCloudDataProvider();
		const targetProject = projectProp ?? params.project!;
		const organization = params.organization!;

		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		return useMutation({
			mutationKey: ["namespaces"],
			mutationFn: async (data: { displayName: string }) => {
				const response = await orgDataProvider.client.namespaces.create(
					targetProject,
					{ displayName: data.displayName, org: organization },
				);
				return {
					id: response.namespace.id,
					name: response.namespace.name,
					displayName: response.namespace.displayName,
					createdAt: new Date(response.namespace.createdAt).toISOString(),
				};
			},
			onSuccess: async (data) => {
				await queryClient.refetchQueries(
					orgDataProvider.currentOrgProjectNamespacesQueryOptions({
						project: targetProject,
					}),
				);
				await navigate({
					to: "/orgs/$organization/projects/$project/ns/$namespace",
					params: {
						organization,
						project: targetProject,
						namespace: data.name,
					},
				});
			},
		});
	}

	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	const dataProvider = match(useRouteContext({ from: "/_context" }))
		.with({ __type: "engine" }, (ctx) => ctx.dataProvider)
		.otherwise(() => {
			throw new Error("Invalid context");
		});

	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	return useMutation(
		dataProvider.createNamespaceMutationOptions({
			onSuccess: async (data) => {
				await queryClient.refetchQueries(
					dataProvider.namespacesQueryOptions(),
				);
				await navigate({
					to: "/ns/$namespace",
					params: { namespace: data.name },
				});
			},
		}),
	);
};

export default function CreateNamespacesFrameContent({
	project,
}: {
	project?: string;
}) {
	const { mutateAsync } = useCreateNamespace({ project });

	return (
		<CreateNamespaceForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
				});
			}}
			defaultValues={{ name: "", slug: "" }}
		>
			<Frame.Header>
				<Frame.Title>Create Namespace</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateNamespaceForm.Name />
					<CreateNamespaceForm.Slug />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateNamespaceForm.Submit type="submit">
					Create
				</CreateNamespaceForm.Submit>
			</Frame.Footer>
		</CreateNamespaceForm.Form>
	);
}
