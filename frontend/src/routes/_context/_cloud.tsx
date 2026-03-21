import {
	createFileRoute,
	notFound,
	Outlet,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { useDialog } from "@/app/use-dialog";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/_context/_cloud")({
	component: RouteComponent,
	beforeLoad: ({ context }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				await waitForClerk(context.clerk);
			})
			.otherwise(() => {
				throw notFound();
			});
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<CloudModals />
		</>
	);
}

function CloudModals() {
	const navigate = useNavigate();
	const search = useSearch({ strict: false });

	const CreateProjectDialog = useDialog.CreateProject.Dialog;

	const CreateOrganizationDialog = useDialog.CreateOrganization.Dialog;

	return (
		<>
			<CreateProjectDialog
				organization={search?.organization}
				dialogProps={{
					open: search?.modal === "create-project",
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
			<CreateOrganizationDialog
				dialogProps={{
					open: search?.modal === "create-organization",
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
