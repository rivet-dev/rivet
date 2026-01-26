import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { RouteError } from "@/app/route-error";
import { useDialog } from "@/app/use-dialog";
import { FullscreenLoading } from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project",
)({
	component: RouteComponent,
	beforeLoad: ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, (context) => {
				return {
					dataProvider: context.getOrCreateProjectContext(
						context.dataProvider,
						params.organization,
						params.project,
					),
				};
			})
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	errorComponent: RouteError,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: FullscreenLoading,
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
