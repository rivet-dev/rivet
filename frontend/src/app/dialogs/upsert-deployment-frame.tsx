import {
	useMutation,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { Button, Code, Frame } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import type { DialogContentProps } from "@/components/hooks";

interface UpsertDeploymentFrameContentProps extends DialogContentProps {
	namespace: string;
	defaultImage?: { repository: string; tag: string };
}

export default function UpsertDeploymentFrameContent({
	namespace: ns,
	defaultImage,
	onClose,
}: UpsertDeploymentFrameContentProps) {
	const dataProvider = useCloudNamespaceDataProvider();
	const queryClient = useQueryClient();

	const { data: namespace } = useSuspenseQuery(
		dataProvider.currentProjectNamespaceQueryOptions({ namespace: ns }),
	);

	const { mutate, isPending } = useMutation({
		...dataProvider.upsertCurrentProjectManagedPoolMutationOptions(),
		onSuccess: async (_data, variables) => {
			await queryClient.invalidateQueries(
				dataProvider.currentProjectManagedPoolQueryOptions({
					namespace: variables.namespace,
					pool: variables.pool,
				}),
			);
			onClose?.();
		},
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title>Deploy to {namespace.displayName}</Frame.Title>
				<Frame.Description>
					{defaultImage ? (
						<>
							Are you sure you want to deploy{" "}
							<Code>
								{defaultImage.repository}:{defaultImage.tag}
							</Code>{" "}
							to <Code>{namespace.displayName}</Code>?
						</>
					) : (
						<>
							Are you sure you want to deploy to{" "}
							<Code>{namespace.displayName}</Code>?
						</>
					)}
				</Frame.Description>
			</Frame.Header>
			<Frame.Footer>
				<Button
					isLoading={isPending}
					onClick={() =>
						mutate({
							namespace: ns,
							displayName: "default",
							pool: "default",
							image: defaultImage,
							minCount: 0,
							maxCount: 100_000,
							environment: {},
							command: undefined,
							args: [],
						})
					}
				>
					Deploy
				</Button>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
			</Frame.Footer>
		</>
	);
}
