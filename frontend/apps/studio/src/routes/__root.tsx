import { FEEDBACK_FORM_ID, FullscreenLoading } from "@rivet-gg/components";

import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { zodValidator } from "@tanstack/zod-adapter";
import { usePostHog } from "posthog-js/react";
import { z } from "zod";
import * as Layout from "@/components/layout";
import { Suspense } from "react";
import { useDialog } from "@rivet-gg/components/actors";

function Modals() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();

	const posthog = usePostHog();

	const FeedbackDialog = useDialog.Feedback.Dialog;
	const GoToActorDialog = useDialog.GoToActor.Dialog;
	const CreateActorDialog = useDialog.CreateActor.Dialog;

	const { modal, utm_source } = search;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: (old) => ({ ...old, modal: undefined }) });
		} else {
			posthog.capture("survey shown", { $survey_id: FEEDBACK_FORM_ID });
		}
	};

	return (
		<>
			<FeedbackDialog
				source={utm_source}
				dialogProps={{
					open: modal === "feedback",
					onOpenChange: handleOnOpenChange,
				}}
			/>
			<GoToActorDialog
				onSubmit={(actorId) => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							actorId,
							modal: undefined,
						}),
					});
				}}
				dialogProps={{
					open: modal === "go-to-actor",
					onOpenChange: (value) => {
						if (!value) {
							navigate({ search: { modal: undefined } });
						}
					},
				}}
			/>
			<CreateActorDialog
				dialogProps={{
					open: modal === "create-actor",
					onOpenChange: (value) => {
						if (!value) {
							navigate({ search: { modal: undefined } });
						}
					},
				}}
			/>
		</>
	);
}

// function RootNotFoundComponent() {
// 	return (
// 		<PageLayout.Root>
// 			<Page title="Not found">
// 				<NotFoundComponent />
// 			</Page>
// 		</PageLayout.Root>
// 	);
// }

// function RootErrorComponent(props: ErrorComponentProps) {
// 	return (
// 		<PageLayout.Root>
// 			<Page title="Error!">
// 				<ErrorComponent {...props} />
// 			</Page>
// 		</PageLayout.Root>
// 	);
// }

function Root() {
	return (
		<Layout.Root>
			<Layout.VisibleInFull>
				<Layout.Header />
				<Layout.Main>
					{/* <Modals /> */}
					<Outlet />
				</Layout.Main>
			</Layout.VisibleInFull>
			<Layout.Footer />
		</Layout.Root>
	);
}

function RootRoute() {
	return (
		<>
			<Root />
			<Suspense>
				<Modals />
			</Suspense>
			{import.meta.env.DEV ? <TanStackRouterDevtools /> : null}
		</>
	);
}

const searchSchema = z.object({
	modal: z.enum(["go-to-actor", "feedback"]).or(z.string()).optional(),
	utm_source: z.string().optional(),
});

export const Route = createRootRouteWithContext()({
	validateSearch: zodValidator(searchSchema),
	component: RootRoute,
	pendingComponent: FullscreenLoading,
	wrapInSuspense: true,
});
