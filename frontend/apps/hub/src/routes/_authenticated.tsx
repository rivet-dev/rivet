import { useAuth } from "@/domains/auth/contexts/auth";
import { useDialog } from "@/hooks/use-dialog";
import { FullscreenLoading } from "@rivet-gg/components";
import * as Layout from "@/layouts/page-centered";
import {
	Outlet,
	createFileRoute,
	retainSearchParams,
	useNavigate,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";
import { LoginView } from "@/domains/auth/views/login-view/login-view";

function Authenticated() {
	const auth = useAuth();
	const navigate = useNavigate();

	if (auth.isProfileLoading) {
		return <FullscreenLoading />;
	}

	if (!auth.profile?.identity.isRegistered) {
		return (
			<Layout.Root>
				<LoginView
					onSuccess={async () => {
						await auth.refreshToken();
						await navigate({
							to: "/",
						});
					}}
				/>
			</Layout.Root>
		);
	}
	return (
		<>
			<Modals />
			<Outlet />
		</>
	);
}

const searchSchema = z.object({
	modal: z.enum(["create-project", "create-group"]).or(z.string()).optional(),
	groupId: z.string().optional().catch(undefined),
});

export const Route = createFileRoute("/_authenticated")({
	validateSearch: zodValidator(searchSchema),
	component: Authenticated,
	search: {
		middlewares: [retainSearchParams(["modal"])],
	},
});

function Modals() {
	const navigate = Route.useNavigate();
	const search = Route.useSearch();

	const CreateGroupProjectDialog = useDialog.CreateProject.Dialog;
	const CreateGroupDialog = useDialog.CreateGroup.Dialog;

	const { groupId, modal } = search;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
		}
	};
	return (
		<>
			<CreateGroupProjectDialog
				groupId={groupId}
				onSuccess={async (data) =>
					await navigate({
						to: "/projects/$projectNameId",
						params: { projectNameId: data.nameId },
						search: { modal: undefined },
					})
				}
				dialogProps={{
					open: modal === "create-project",
					onOpenChange: handleOnOpenChange,
				}}
			/>
			<CreateGroupDialog
				onSuccess={async (data) =>
					await navigate({
						to: "/teams/$groupId",
						params: { groupId: data.groupId },
						search: { modal: undefined },
					})
				}
				dialogProps={{
					open: modal === "create-group",
					onOpenChange: handleOnOpenChange,
				}}
			/>
		</>
	);
}
