import {
	createFileRoute,
	Outlet,
	redirect,
	useNavigate,
	useParams,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { match } from "ts-pattern";
import z from "zod";
import { createGlobalContext as createGlobalCloudContext } from "@/app/data-providers/cloud-data-provider";
import { createGlobalContext as createGlobalEngineContext } from "@/app/data-providers/engine-data-provider";
import { createGlobalContext as createGlobalInspectorContext } from "@/app/data-providers/inspector-data-provider";
import { getConfig, ls, useDialog } from "@/components";
import { ModalRenderer } from "@/components/modal-renderer";
import { waitForClerk } from "@/lib/waitForClerk";

const searchSchema = z
	.object({
		modal: z
			.enum([
				"go-to-actor",
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
		// clerk related
		__clerk_ticket: z.string().optional(),
		__clerk_status: z.string().optional(),
	})
	.and(z.record(z.string(), z.any()));

export const Route = createFileRoute("/_context")({
	component: RouteComponent,
	validateSearch: zodValidator(searchSchema),
	context: ({ location: { search }, context }) => {
		return match(__APP_TYPE__)
			.with("engine", () => ({
				dataProvider: createGlobalEngineContext({
					engineToken: () =>
						ls.engineCredentials.get(getConfig().apiUrl) || "",
				}),
				__type: "engine" as const,
			}))
			.with("cloud", () => ({
				dataProvider: createGlobalCloudContext({
					clerk: context.clerk,
				}),
				__type: "cloud" as const,
			}))
			.with("inspector", () => ({
				dataProvider: createGlobalInspectorContext({
					url: (search as z.infer<typeof searchSchema>).u,
					token: (search as z.infer<typeof searchSchema>).t,
				}),
				__type: "inspector" as const,
			}))
			.exhaustive();
	},
	beforeLoad: async (route) => {
		return await match(route.context)
			.with({ __type: "cloud" }, () => async () => {
				await waitForClerk(route.context.clerk);

				if (
					route.search.__clerk_ticket &&
					route.search.__clerk_status
				) {
					throw redirect({
						to: "/onboarding/accept-invitation",
						search: { ...route.search },
					});
				}

				if (!route.context.clerk.user) {
					throw redirect({
						to: "/login",
						search: { from: location.pathname },
					});
				}
			})
			.otherwise(() => () => {})();
	},
	loaderDeps: (route) => ({ token: route.search.t, url: route.search.u }),
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
	const params = useParams({ strict: false });

	const CreateActorDialog = useDialog.CreateActor.Dialog;
	const FeedbackDialog = useDialog.Feedback.Dialog;

	return (
		<>
			<CreateActorDialog
				dialogProps={{
					open: search.modal === "create-actor",
					onOpenChange: (value) => {
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
			<FeedbackDialog
				dialogProps={{
					open: search.modal === "feedback",
					onOpenChange: (value) => {
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
