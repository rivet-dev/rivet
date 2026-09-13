import { createFileRoute } from "@tanstack/react-router";
import { SidebarlessHeader } from "@/app/layout";

export const Route = createFileRoute(
	"/_context/orgs/$organization/clusters/$cluster",
)({
	component: RouteComponent,
});

// Placeholder BYOC cluster page. Cluster setup UI lands here later.
function RouteComponent() {
	const { cluster } = Route.useParams();

	return (
		<div className="h-screen flex flex-col overflow-hidden">
			<SidebarlessHeader />
			<div className="flex-1 min-h-0 flex items-center justify-center">
				<div className="text-center">
					<h1 className="text-2xl font-semibold">{cluster}</h1>
					<p className="text-muted-foreground mt-1">
						Cluster created. Setup coming soon.
					</p>
				</div>
			</div>
		</div>
	);
}
