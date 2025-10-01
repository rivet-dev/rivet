import { faRefresh, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { RunnersTable } from "@/app/runners-table";
import {
	Button,
	H1,
	WithTooltip,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/runners")(
	{
		component: RouteComponent,
	},
);

function RouteComponent() {
	const { namespace } = Route.useParams();
	const {
		data: runners,
		isRefetching,
		hasNextPage,
		fetchNextPage,
		isLoading,
		isError,
		refetch,
	} = useInfiniteQuery(
		useEngineCompatDataProvider().runnersQueryOptions({
			namespace: namespace,
		}),
	);

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="max-w-5xl mx-auto my-2 flex justify-between items-center px-6 py-4">
				<H1>Runners</H1>
				<div className="flex items-center gap-2">
					<WithTooltip
						content="Refresh"
						trigger={
							<Button
								size="icon"
								isLoading={isRefetching}
								variant="outline"
								onClick={() => refetch()}
							>
								<Icon icon={faRefresh} />
							</Button>
						}
					/>
				</div>
			</div>

			<hr className="mb-4" />

			<div className="p-4">
				<div className="max-w-5xl mx-auto p-2">
					<div className="border rounded-md">
						<RunnersTable
							isLoading={isLoading}
							isError={isError}
							runners={runners || []}
							fetchNextPage={fetchNextPage}
							hasNextPage={hasNextPage}
						/>
					</div>
				</div>
			</div>
		</div>
	);
}
