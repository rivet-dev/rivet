import { createFileRoute } from "@tanstack/react-router";
import { GettingStarted } from "@/app/getting-started";
import { RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/",
)({
	component: RouteComponent,
});

function RouteComponent() {
	const navigate = Route.useNavigate();
	const search = Route.useSearch();
	const StartWithTemplateDialog = useDialog.StartWithTemplate.Dialog;
	return (
		<>
			<RouteLayout>
				<GettingStarted
					showFooter={false}
					getTemplateLink={(slug) => ({
						to: "/orgs/$organization/new",
						search: { template: slug, modal: "get-started" },
					})}
				/>
			</RouteLayout>
			<StartWithTemplateDialog
				name={search.template}
				provider={search.provider}
				createProjectOnProviderSelect
				dialogProps={{
					open: search.modal === "get-started",
					onOpenChange: (value) => {
						if (!value) {
							return navigate({
								to: ".",
								search: (old) => ({
									...old,
									modal: undefined,
								}),
							});
						}
					},
				}}
			/>
		</>
	);
}
