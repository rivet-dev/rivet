import {} from "@fortawesome/free-solid-svg-icons";
import { templates } from "@rivetkit/example-registry";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { SidebarlessHeader } from "@/app/layout";
import { TemplateDetail } from "@/components/template-detail";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/$template",
)({
	component: RouteComponent,
	loader: async (context) => {
		const { template } = context.params;

		const templateData = templates.find((t) => t.name === template);
		if (!templateData) {
			throw redirect({
				to: "/orgs/$organization/new",
				params: context.params,
				search: true,
			});
		}

		return {
			template: templateData,
		};
	},
	loaderDeps(opts) {
		return [];
	},
});

function RouteComponent() {
	const { template } = Route.useLoaderData();
	const { organization } = Route.useParams();

	return (
		<>
			<SidebarlessHeader />
			<TemplateDetail template={template} organization={organization} />
		</>
	);
}
