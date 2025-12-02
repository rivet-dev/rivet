import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
	useNavigate,
	useParams,
	useRouteContext,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import * as CreateNamespaceForm from "@/app/forms/create-namespace-form";
import { Flex, Frame } from "@/components";
import { convertStringToId } from "@/lib/utils";

const useDataProvider = () => {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: match will only run once per app load
			return useRouteContext({
				from: "/_context/_cloud/orgs/$organization/projects/$project",
				select: (ctx) => ctx.dataProvider,
			});
		})
		.with("engine", () => {
			return match(
				// biome-ignore lint/correctness/useHookAtTopLevel: match will only run once per app load
				useRouteContext({
					from: "/_context",
				}),
			)
				.with({ __type: "engine" }, (ctx) => ctx.dataProvider)
				.otherwise(() => {
					throw new Error("Invalid context");
				});
		})
		.otherwise(() => {
			throw new Error("Invalid app type");
		});
};

const useCreateNamespace = () => {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const params = useParams({ strict: false });

	const dataProivder = useDataProvider();

	return useMutation(
		dataProivder.createNamespaceMutationOptions({
			onSuccess: async (data) => {
				// Invalidate all queries to ensure fresh data
				await queryClient.invalidateQueries(
					dataProivder.namespacesQueryOptions(),
				);

				if (__APP_TYPE__ === "cloud") {
					if (!params.project || !params.organization) {
						throw new Error("Missing required parameters");
					}
					// Navigate to the newly created namespace
					await navigate({
						to: "/orgs/$organization/projects/$project/ns/$namespace",
						params: {
							organization: params.organization,
							project: params.project,
							namespace: data.name,
						},
					});
					return;
				}

				await navigate({
					to: "/ns/$namespace",
					params: { namespace: data.name },
				});
			},
		}),
	);
};

export default function CreateNamespacesFrameContent() {
	const { mutateAsync } = useCreateNamespace();

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
