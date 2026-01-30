import {
	createFileRoute,
	isRedirect,
	Outlet,
	redirect,
	useNavigate,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { match } from "ts-pattern";
import z from "zod";
import { getInspectorClientEndpoint } from "@/app/data-providers/inspector-data-provider";
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
				dataProvider: context.getOrCreateEngineContext(
					() => ls.engineCredentials.get(getConfig().apiUrl) || "",
				),
				__type: "engine" as const,
			}))
			.with("cloud", () => ({
				dataProvider: context.getOrCreateCloudContext(context.clerk),
				__type: "cloud" as const,
			}))
			.with("inspector", () => {
				const typedSearch = search as z.infer<typeof searchSchema>;
				return {
					dataProvider: context.getOrCreateInspectorContext({
						url: typedSearch.u || "http://localhost:6420",
						token: typedSearch.t,
					}),
					__type: "inspector" as const,
				};
			})
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
						search: true,
					});
				}

				if (!route.context.clerk.user) {
					throw redirect({
						to: "/login",
						search: (old) => ({
							...old,
							from: route.location.pathname,
						}),
					});
				}
			})
			.with({ __type: "inspector" }, () => async () => {
				if (route.search.u) {
					try {
						const realUrl = await getInspectorClientEndpoint(
							route.search.u,
						);
						if (realUrl !== route.search.u) {
							throw redirect({
								to: route.location.pathname,
								search: {
									...route.search,
									u: realUrl,
								},
							});
						}
					} catch (e) {
						if (isRedirect(e)) {
							throw e;
						}
						// ignore errors here
					}
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

	const CreateActorDialog = useDialog.CreateActor.Dialog;
	const FeedbackDialog = useDialog.Feedback.Dialog;

	return (
		<>
			<CreateActorDialog
				dialogProps={{
					open: search.modal === "create-actor",
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
		</>
	);
}
