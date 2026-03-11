import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import {
	queryOptions,
	useQueries,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { HelpDropdown } from "@/app/help-dropdown";
import { ImagesTable } from "@/app/images-table";
import { Content } from "@/app/layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1 } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/deployments",
)({
	component: RouteComponent,
	loader: async ({ context }) => {
		const dataProvider = context.dataProvider;
		await Promise.all([
			context.queryClient.prefetchInfiniteQuery({
				...dataProvider.currentProjectNamespacesQueryOptions(),
				pages: Infinity,
			}),
			context.queryClient.prefetchInfiniteQuery({
				...dataProvider.currentProjectImageRepositoriesQueryOptions(),
				pages: Infinity,
			}),
		]);
	},
	loaderDeps() {
		return [];
	},
});

function RouteComponent() {
	return (
		<Content>
			<div className=" ">
				<div className="mb-4 pt-2 max-w-5xl mx-auto">
					<div className="flex justify-between items-center px-6 @6xl:px-0 py-4 ">
						<SidebarToggle className="absolute left-4" />
						<H1>Deployments</H1>
						<HelpDropdown>
							<Button
								variant="outline"
								startIcon={<Icon icon={faQuestionCircle} />}
							>
								Need help?
							</Button>
						</HelpDropdown>
					</div>
					<p className="max-w-5xl mb-6 px-6 @6xl:px-0 text-muted-foreground">
						Deployments are live instances of your application
						running in the cloud. Create, monitor, and manage your
						deployments here.
					</p>
				</div>

				<hr className="mb-6" />

				<div className="px-4">
					<Deployments />
				</div>
			</div>
		</Content>
	);
}

function Deployments() {
	const { namespace } = Route.useParams();
	const dataProvider = useCloudNamespaceDataProvider();
	const {
		data: images,
		isError,
		isLoading: isLoadingImages,
		fetchNextPage,
		hasNextPage,
	} = useSuspenseInfiniteQuery(
		dataProvider.currentProjectImagesQueryOptions(),
	);

	const { data: namespaces } = useSuspenseInfiniteQuery(
		dataProvider.currentProjectNamespacesQueryOptions(),
	);

	const managedPoolQueries = useQueries({
		queries:
			namespaces.map((ns) =>
				queryOptions({
					...dataProvider.currentProjectManagedPoolQueryOptions({
						namespace: ns.name,
						pool: "default",
					}),
					select: (data) => ({
						...data,
						namespace: ns.name,
						...data.config.image,
					}),
				}),
			) ?? [],
	});

	const deployments = managedPoolQueries
		.map((query) => query.data)
		.filter(
			(data): data is Exclude<typeof data, undefined> =>
				data !== undefined,
		);

	const isLoading =
		managedPoolQueries.some((query) => query.isLoading) || isLoadingImages;

	return (
		<div className="max-w-5xl mx-auto">
			<div className="border rounded-md">
				<ImagesTable
					images={isLoading ? [] : images}
					deployments={isLoading ? [] : deployments}
					namespace={namespace}
					isLoading={isLoading}
					isError={isError}
					fetchNextPage={fetchNextPage}
					hasNextPage={hasNextPage}
				/>
			</div>
		</div>
	);
}
