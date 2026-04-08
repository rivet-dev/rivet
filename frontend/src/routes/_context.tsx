import {
	createFileRoute,
	Outlet,
	redirect,
	useNavigate,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import z from "zod";
import { getConfig, ls } from "@/components";
import { useDialog } from "@/app/use-dialog";
import { ModalRenderer } from "@/components/modal-renderer";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

const searchSchema = z
	.object({
		modal: z
			.enum([
				"feedback",
				"create-ns",
				"create-project",
				"billing",
			])
			.or(z.string())
			.optional(),
		utm_source: z.string().optional(),
		actorId: z.string().optional(),
		tab: z.string().optional(),
		n: z.array(z.string()).optional(),
		u: z.string().optional(),
		t: z.string().optional(),
	})
	.and(z.record(z.string(), z.any()));

export const Route = createFileRoute("/_context")({
	component: RouteComponent,
	validateSearch: zodValidator(searchSchema),
	context: ({ context }) => {
		if (features.multitenancy) {
			return {
				dataProvider: context.getOrCreateCloudContext(),
				__type: "cloud" as const,
			};
		}
		return {
			dataProvider: context.getOrCreateEngineContext(
				() => ls.engineCredentials.get(getConfig().apiUrl) || "",
			),
			__type: "engine" as const,
		};
	},
	beforeLoad: async (route) => {
		if (features.multitenancy) {
			const session = await authClient.getSession();

			if (!session.data) {
				throw redirect({
					to: "/login",
					search: (old) => ({
						...old,
						from: route.location.pathname,
					}),
				});
			}
		}
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<ModalRenderer />
			<Modals />
			{!features.multitenancy && <EngineModals />}
		</>
	);
}

function Modals() {
	const navigate = useNavigate();
	const search = Route.useSearch();

	const FeedbackDialog = useDialog.Feedback.Dialog;

	return (
		<FeedbackDialog
			dialogProps={{
				open: search.modal === "feedback",
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
	);
}

function EngineModals() {
	const navigate = useNavigate();
	const search = Route.useSearch();
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;
	return (
		<CreateNamespaceDialog
			dialogProps={{
				open: search.modal === "create-ns",
				onOpenChange: (value) => {
					if (!value) {
						return navigate({
							to: ".",
							search: (old) => ({ ...old, modal: undefined }),
						});
					}
				},
			}}
		/>
	);
}
