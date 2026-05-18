import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import { createFileRoute, notFound } from "@tanstack/react-router";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { NamespaceSettingsContent } from "@/app/settings-pages/namespace-settings";
import { Button, H1, H2, Skeleton } from "@/components";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/ns/$namespace/settings",
)({
	component: RouteComponent,
	pendingComponent: DataLoadingPlaceholder,
	beforeLoad: async () => {
		if (!features.multitenancy) {
			throw notFound();
		}
	},
	loader: async ({ context }) => {
		const dataProvider = context.dataProvider;
		await Promise.all([
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnerConfigsQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnersQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.currentNamespaceEnvoyListQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.datacentersQueryOptions(),
			),
			context.queryClient.prefetchQuery(
				dataProvider.currentProjectQueryOptions(),
			),
			context.queryClient.prefetchQuery(
				dataProvider.currentNamespaceQueryOptions(),
			),
		]);
	},
	loaderDeps() {
		return [];
	},
});

export function RouteComponent() {
	return (
		<Content>
			<div className="mb-4 pt-2 max-w-5xl mx-auto">
				<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
					<H1>Settings</H1>
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
					Connect your RivetKit application to Rivet Cloud. Use your
					cloud of choice to run Rivet Actors.
				</p>
			</div>

			<hr className="mb-6" />

			<div className="max-w-5xl mx-auto px-6 @6xl:px-0">
				<NamespaceSettingsContent />
			</div>
		</Content>
	);
}

function DataLoadingPlaceholder() {
	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="mt-2 flex justify-between items-center px-6 py-4 max-w-5xl mx-auto">
				<H2 className="mb-2">
					<Skeleton className="w-48 h-8" />
				</H2>
			</div>
			<p className="max-w-5xl mb-6 px-6 text-muted-foreground mx-auto">
				<Skeleton className="w-full h-4" />
			</p>
			<hr className="mb-4" />
			<div className="p-4 px-6 max-w-5xl mx-auto ">
				<Skeleton className="h-8 w-48 mb-2" />
				<Skeleton className="h-6 w-72 mb-6" />
				<div className="flex flex-wrap gap-2 my-4">
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
					<Skeleton className="w-full h-20 rounded-md" />
				</div>
			</div>
		</div>
	);
}
