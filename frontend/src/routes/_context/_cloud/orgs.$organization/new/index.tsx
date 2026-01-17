import { createFileRoute } from "@tanstack/react-router";
import { SidebarlessHeader } from "@/app/layout";
import { TemplatesList } from "@/components/templates-list";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<>
			<SidebarlessHeader />
			<TemplatesList
				showBackHome={false}
				getTemplateLink={(template) => ({
					to: "/orgs/$organization/new/$template",
					params: { template },
				})}
				startFromScratchLink={{
					to: ".",
					search: {
						modal: "create-project",
					},
				}}
			/>
		</>
	);
}
