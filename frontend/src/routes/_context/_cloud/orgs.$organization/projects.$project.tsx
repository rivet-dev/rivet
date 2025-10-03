import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { createProjectContext } from "@/app/data-providers/cloud-data-provider";
import { RouteError } from "@/app/route-error";
import { useDialog } from "@/app/use-dialog";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project",
)({
	component: RouteComponent,
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, (context) => ({
				dataProvider: {
					...context.dataProvider,
					...createProjectContext({
						...context.dataProvider,
						organization: params.organization,
						project: params.project,
					}),
				},
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	errorComponent: RouteError,
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<ProjectModals />
		</>
	);
}

function ProjectModals() {
	const navigate = Route.useNavigate();
	const search = Route.useSearch();

	const BillingDialog = useDialog.Billing.Dialog;

	return (
		<>
			<BillingDialog
				dialogContentProps={{
					className: "max-w-5xl",
				}}
				dialogProps={{
					open: search.modal === "billing",
					// FIXME
					onOpenChange: (value: any) => {
						if (!value) {
							navigate({
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
