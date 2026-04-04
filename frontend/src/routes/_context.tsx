import {
	createFileRoute,
	Outlet,
	redirect,
	useNavigate,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { match } from "ts-pattern";
import z from "zod";
import { getConfig, ls, useDialog } from "@/components";
import { ModalRenderer } from "@/components/modal-renderer";
import { authClient } from "@/lib/auth";

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
		return match(__APP_TYPE__)
			.with("engine", () => ({
				dataProvider: context.getOrCreateEngineContext(
					() => ls.engineCredentials.get(getConfig().apiUrl) || "",
				),
				__type: "engine" as const,
			}))
			.with("cloud", () => ({
				dataProvider: context.getOrCreateCloudContext(),
				__type: "cloud" as const,
			}))
			.otherwise(() => {
				throw new Error(
					"Inspector routes are not supported in the dashboard build",
				);
			});
	},
	beforeLoad: async (route) => {
		return await match(route.context)
			.with({ __type: "cloud" }, () => async () => {
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
			})
			.otherwise(() => () => {})();
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<ModalRenderer />
			<Modals />
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
